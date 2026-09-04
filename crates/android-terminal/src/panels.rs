use std::collections::{HashMap, VecDeque};

use adb_client::{NetworkStats, ProtocolStats, StorageCategory, StorageOverview};
use eframe::egui;

use crate::app::{App, CachedLogLine, InsightStatus, LogcatTagFilter};
use crate::theme;

#[derive(Default)]
pub struct AppStorageState {
    pub packages: Vec<String>,
    pub sizes: HashMap<String, u64>,
    pub scanning: bool,
    pub error: Option<String>,
}

impl AppStorageState {
    pub fn set_packages(&mut self, packages: Vec<String>) {
        self.packages = packages;
        self.sizes.clear();
        self.scanning = true;
    }

    pub fn merge_packages(&mut self, packages: Vec<String>) {
        self.packages = packages;
        self.sizes
            .retain(|package, _| self.packages.iter().any(|pkg| pkg == package));
        self.scanning = true;
    }

    pub fn set_size(&mut self, package: &str, bytes: u64) {
        self.sizes.insert(package.to_string(), bytes);
    }

    pub fn sorted_rows(&self) -> Vec<(&str, Option<u64>)> {
        let mut rows: Vec<(&str, Option<u64>)> = self
            .packages
            .iter()
            .map(|pkg| (pkg.as_str(), self.sizes.get(pkg).copied()))
            .collect();
        rows.sort_by(|a, b| match (a.1, b.1) {
            (Some(left), Some(right)) => right.cmp(&left),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.cmp(b.0),
        });
        rows
    }
}

pub fn logcat_all(ui: &mut egui::Ui, app: &mut App, auto_scroll: bool) {
    theme::filter_row(ui, |ui| {
        ui.label("Filter:");
        ui.add(
            egui::TextEdit::singleline(&mut app.logcat_filter)
                .hint_text("Search logs…")
                .desired_width(ui.available_width()),
        )
        .on_hover_text("Filter log lines by text");
    });

    theme::filter_row(ui, |ui| {
        ui.label("Tag:");
        let response = ui
            .add(
                egui::TextEdit::singleline(&mut app.logcat_tag_input)
                    .hint_text("Add tag…")
                    .desired_width(ui.available_width())
                    .id_salt("logcat_tag_input"),
            )
            .on_hover_text("Press Enter to add a tag filter");
        if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            app.add_logcat_tag();
            response.request_focus();
        }
    });

    let mut remove_tag_index = None;
    theme::tag_filter_row(ui, &app.logcat_tag_filters, &mut remove_tag_index);
    if let Some(index) = remove_tag_index {
        app.remove_logcat_tag(index);
    }

    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if let Some(error) = &app.logcat_error {
        theme::error_label(ui, error);
    }

    if app.logcat_rx.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if app.log_lines.is_empty() {
        ui.label("Waiting for log output…");
        return;
    }

    let filter = app.logcat_filter.clone();
    let tag_filters = app.logcat_tag_filters.clone();
    let show_timestamps = app.logcat_show_timestamps;
    let matching = filtered_line_indices(&app.log_lines, &filter, &tag_filters, show_timestamps);

    show_log_scroll(
        ui,
        &app.log_lines,
        &matching,
        auto_scroll,
        show_timestamps,
        app.logcat_line_spacing,
        egui::Id::new("logcat_all_scroll"),
        LogScrollStyle::ByLevel,
        Some(&tag_filters),
    );
}

pub fn insight(ui: &mut egui::Ui, app: &mut App, auto_scroll: bool) {
    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    theme::panel_body(ui, theme::colors::INSIGHT_BODY, |ui| {
        if app.insight.replies.is_empty() {
            match app.insight.status {
                InsightStatus::RequestFailed => {
                    theme::error_label(ui, "request failed, see log");
                }
                InsightStatus::Idle | InsightStatus::RequestSent => {
                    ui.label("...");
                }
            }
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt(egui::Id::new("insight_scroll"))
            .stick_to_bottom(auto_scroll)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for (index, reply) in app.insight.replies.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }
                    ui.label(reply.as_str());
                }
            });
    });
}

pub fn logcat_errors(ui: &mut egui::Ui, app: &mut App, auto_scroll: bool) {
    theme::filter_row(ui, |ui| {
        ui.label("Filter:");
        ui.add(
            egui::TextEdit::singleline(&mut app.error_logcat_filter)
                .hint_text("Search errors…")
                .desired_width(ui.available_width()),
        )
        .on_hover_text("Filter error lines by text");
    });

    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if let Some(error) = &app.error_logcat_error {
        theme::error_label(ui, error);
    }

    if app.error_logcat_rx.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if app.error_lines.is_empty() {
        ui.label("Waiting for error log output…");
        return;
    }

    let filter = app.error_logcat_filter.clone();
    let matching = filtered_line_indices_simple(&app.error_lines, &filter);

    show_log_scroll(
        ui,
        &app.error_lines,
        &matching,
        auto_scroll,
        app.error_show_timestamps,
        app.error_line_spacing,
        egui::Id::new("logcat_errors_scroll"),
        LogScrollStyle::ErrorsOnly,
        None,
    );
}

pub fn storage_usage(ui: &mut egui::Ui, app: &App) {
    if let Some(error) = &app.storage_breakdown_error {
        theme::error_label(ui, error);
    }
    if let Some(error) = &app.app_storage.error {
        theme::error_label(ui, error);
    }

    theme::panel_body(ui, theme::colors::MEMORY_DISK_BODY, |ui| {
        if app.selected_serial.is_none() {
            theme::panel_loading(ui);
            return;
        }

        egui::ScrollArea::both()
            .id_salt(egui::Id::new("storage_usage_scroll"))
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                if let Some(breakdown) = &app.storage_breakdown {
                    show_storage_categories(ui, &breakdown.categories);
                } else if app.storage_breakdown_rx.is_some() {
                    ui.label("Loading storage…");
                }
                ui.separator();
                show_app_storage(ui, &app.app_storage);
            });
    });
}

pub fn ram_gauge(ui: &mut egui::Ui, app: &App) {
    theme::panel(ui, "RAM", |ui| {
        if app.selected_serial.is_none() {
            theme::panel_loading(ui);
            return;
        }

        if let Some(error) = &app.ram_error {
            theme::error_label(ui, error);
        }

        let Some(memory) = &app.ram_memory else {
            if app.ram_rx.is_some() {
                theme::panel_loading(ui);
            } else {
                theme::panel_loading(ui);
            }
            return;
        };

        show_ram_donut(ui, memory);
    });
}

pub fn storage_gauge(ui: &mut egui::Ui, app: &App) {
    theme::panel(ui, "Storage", |ui| {
        if app.selected_serial.is_none() {
            theme::panel_loading(ui);
            return;
        }

        if let Some(error) = &app.storage_gauge_error {
            theme::error_label(ui, error);
        }

        let Some(overview) = &app.storage_gauge else {
            if app.storage_gauge_rx.is_some() {
                theme::panel_loading(ui);
            } else {
                theme::panel_loading(ui);
            }
            return;
        };

        show_storage_donut(ui, overview);
    });
}

fn show_ram_donut(ui: &mut egui::Ui, memory: &adb_client::MemoryStats) {
    show_usage_donut(
        ui,
        egui::Id::new("ram_gauge"),
        memory.used_fraction(),
        format_gb_from_kb(memory.used_kb()),
        format_gb_from_kb(memory.total_kb),
        theme::colors::RAM_TRACK,
        theme::colors::RAM_USED,
    );
}

fn show_storage_donut(ui: &mut egui::Ui, overview: &StorageOverview) {
    show_usage_donut(
        ui,
        egui::Id::new("storage_gauge"),
        overview.used_fraction(),
        format_gb_from_bytes(overview.used_bytes),
        format_gb_from_bytes(overview.total_bytes),
        theme::colors::STORAGE_TRACK,
        theme::colors::STORAGE_USED,
    );
}

fn show_usage_donut(
    ui: &mut egui::Ui,
    scroll_id: egui::Id,
    fraction: f32,
    used_label: String,
    total_label: String,
    track_color: egui::Color32,
    used_color: egui::Color32,
) {
    let percent = (fraction * 100.0).round() as u32;

    egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.set_width(ui.available_width());

                let diameter = ui.available_width().clamp(80.0, 160.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::hover());

                let painter = ui.painter_at(rect);
                let center = rect.center();
                let radius = diameter * 0.38;
                let stroke_width = diameter * 0.12;

                paint_usage_donut(
                    &painter,
                    center,
                    radius,
                    stroke_width,
                    fraction,
                    track_color,
                    used_color,
                );
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    format!("{percent}%"),
                    egui::FontId::proportional(28.0),
                    theme::colors::OFF_WHITE,
                );

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("{used_label} / {total_label}"))
                        .color(theme::colors::LOG_DEBUG)
                        .size(12.0),
                );
            });
        });
}

fn paint_usage_donut(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    stroke_width: f32,
    fraction: f32,
    track_color: egui::Color32,
    used_color: egui::Color32,
) {
    use std::f32::consts::TAU;

    let start = -TAU / 4.0;
    let used = fraction.clamp(0.0, 1.0);

    paint_ring_arc(
        painter,
        center,
        radius,
        start,
        TAU,
        stroke_width,
        track_color,
    );
    if used > 0.0 {
        paint_ring_arc(
            painter,
            center,
            radius,
            start,
            used * TAU,
            stroke_width,
            used_color,
        );
    }
}

fn paint_ring_arc(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    start: f32,
    sweep: f32,
    stroke_width: f32,
    color: egui::Color32,
) {
    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: arc_points(center, radius, start, sweep, 64),
        closed: false,
        fill: egui::Color32::TRANSPARENT,
        stroke: egui::epaint::PathStroke::new(stroke_width, color),
    }));
}

fn arc_points(
    center: egui::Pos2,
    radius: f32,
    start: f32,
    sweep: f32,
    steps: usize,
) -> Vec<egui::Pos2> {
    (0..=steps)
        .map(|step| {
            let angle = start + sweep * step as f32 / steps as f32;
            center + egui::vec2(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

fn format_gb_from_kb(kb: u64) -> String {
    format!("{:.1} GB", kb as f64 / 1_048_576.0)
}

fn format_gb_from_bytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
}

pub fn network(ui: &mut egui::Ui, app: &App) {
    if let Some(error) = &app.network_error {
        theme::error_label(ui, error);
    }

    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    let Some(stats) = &app.network_stats else {
        if app.network_rx.is_some() {
            ui.label("Fetching network stats…");
        } else {
            theme::panel_loading(ui);
        }
        return;
    };

    if stats.interfaces.is_empty() {
        ui.label("No network interfaces reported.");
        return;
    }

    let rows = network_rows_from_stats(stats);
    show_network_table(ui, &rows);
}

pub fn protocols(ui: &mut egui::Ui, app: &App) {
    if let Some(error) = &app.protocol_error {
        theme::error_label(ui, error);
    }

    theme::panel_body(ui, theme::colors::APP_TRAFFIC_BODY, |ui| {
        if app.selected_serial.is_none() {
            theme::panel_loading(ui);
            return;
        }

        let Some(stats) = &app.protocol_stats else {
            if app.protocol_rx.is_some() {
                ui.label("Fetching app traffic…");
            } else {
                theme::panel_loading(ui);
            }
            return;
        };

        show_protocol_stats(ui, stats);
    });
}

fn show_protocol_stats(ui: &mut egui::Ui, stats: &ProtocolStats) {
    if stats.apps.is_empty() {
        ui.label("No app traffic reported.");
        return;
    }

    egui::ScrollArea::both()
        .id_salt(egui::Id::new("app_traffic_scroll"))
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            egui::Grid::new("app_traffic")
                .num_columns(6)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Package");
                    ui.label("Total");
                    ui.label("Foreground");
                    ui.label("Background");
                    ui.label("WiFi");
                    ui.label("Mobile");
                    ui.end_row();

                    for app in &stats.apps {
                        ui.label(&app.package);
                        ui.label(format_bytes_mb(app.total_bytes));
                        ui.label(format_bytes_mb(app.foreground_bytes));
                        ui.label(format_bytes_mb(app.background_bytes));
                        ui.label(format_bytes_mb(app.wifi_bytes));
                        ui.label(format_bytes_mb(app.mobile_bytes));
                        ui.end_row();
                    }
                });
        });
}

fn show_storage_categories(ui: &mut egui::Ui, categories: &[StorageCategory]) {
    ui.label("Storage breakdown");

    egui::Grid::new("storage_categories")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Category");
            ui.label("Size");
            ui.end_row();

            for category in categories {
                ui.label(&category.name);
                ui.label(format_bytes(category.bytes));
                ui.end_row();
            }
        });
}

fn show_app_storage(ui: &mut egui::Ui, storage: &AppStorageState) {
    ui.horizontal(|ui| {
        ui.label("Apps");
        if storage.scanning {
            ui.label(format!(
                "({}/{})",
                storage.sizes.len(),
                storage.packages.len()
            ));
        }
    });

    if storage.packages.is_empty() {
        ui.label("Loading package list…");
        return;
    }

    egui::Grid::new("app_storage")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Package");
            ui.label("Storage");
            ui.end_row();

            for (package, bytes) in storage.sorted_rows() {
                ui.label(truncate_package_name(package))
                    .on_hover_text(package);
                ui.label(match bytes {
                    Some(value) => format_bytes_mb(value),
                    None => "…".to_string(),
                });
                ui.end_row();
            }
        });
}

fn show_network_table(ui: &mut egui::Ui, stats: &[NetworkRow]) {
    egui::ScrollArea::both()
        .id_salt(egui::Id::new("network_table_scroll"))
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            egui::Grid::new("network_stats")
                .num_columns(5)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Interface");
                    ui.label("Transport");
                    ui.label("RX");
                    ui.label("TX");
                    ui.label("Rate");
                    ui.end_row();

                    for row in stats {
                        ui.label(&row.interface);
                        ui.label(&row.transport);
                        ui.label(&row.rx);
                        ui.label(&row.tx);
                        ui.label(&row.rate);
                        ui.end_row();
                    }
                });
        });
}

/// Display row for the network activity table.
#[derive(Clone)]
pub struct NetworkRow {
    pub interface: String,
    pub transport: String,
    pub rx: String,
    pub tx: String,
    pub rate: String,
}

pub fn network_rows_from_stats(stats: &NetworkStats) -> Vec<NetworkRow> {
    let mut interfaces: Vec<_> = stats.interfaces.iter().collect();
    interfaces.sort_by(|a, b| b.tx_bytes.cmp(&a.tx_bytes));
    interfaces
        .into_iter()
        .map(|iface| NetworkRow {
            interface: iface.interface.clone(),
            transport: iface.transport.clone(),
            rx: format_bytes_mb(iface.rx_bytes),
            tx: format_bytes_mb(iface.tx_bytes),
            rate: format_rate_mb(iface.rx_rate_bps, iface.tx_rate_bps),
        })
        .collect()
}

fn truncate_package_name(name: &str) -> String {
    const MAX_LEN: usize = 42;
    if name.len() <= MAX_LEN {
        return name.to_string();
    }
    format!("{}...", &name[..MAX_LEN - 3])
}

fn format_bytes_mb(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    }
}

fn format_rate_mb(rx_bps: f64, tx_bps: f64) -> String {
    format!(
        "↓ {}  ↑ {}",
        format_throughput(rx_bps),
        format_throughput(tx_bps)
    )
}

fn format_throughput(bps: f64) -> String {
    if bps < 0.0 {
        return "—".to_string();
    }
    if bps >= 1_048_576.0 {
        format!("{:.2} MB/s", bps / 1_048_576.0)
    } else if bps >= 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.0} B/s", bps)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

enum LogScrollStyle {
    ByLevel,
    ErrorsOnly,
}

fn filtered_line_indices(
    lines: &VecDeque<CachedLogLine>,
    text_filter: &str,
    tag_filters: &[LogcatTagFilter],
    show_timestamps: bool,
) -> Vec<usize> {
    let text_filter = text_filter.trim();
    let text_active = !text_filter.is_empty();
    let text_lower = text_filter.to_lowercase();

    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            if text_active && !line.matches_filter(&text_lower) {
                return false;
            }
            line.matches_tag_filters(tag_filters, show_timestamps)
        })
        .map(|(index, _)| index)
        .collect()
}

fn filtered_line_indices_simple(lines: &VecDeque<CachedLogLine>, filter: &str) -> Vec<usize> {
    let filter = filter.trim();
    if filter.is_empty() {
        return (0..lines.len()).collect();
    }

    let filter_lower = filter.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.matches_filter(&filter_lower))
        .map(|(index, _)| index)
        .collect()
}

fn show_log_scroll(
    ui: &mut egui::Ui,
    lines: &VecDeque<CachedLogLine>,
    matching: &[usize],
    stick_to_bottom: bool,
    show_timestamps: bool,
    line_spacing: bool,
    scroll_id: egui::Id,
    style: LogScrollStyle,
    tag_filters: Option<&[LogcatTagFilter]>,
) {
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
    let text_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let row_height = if line_spacing {
        text_height * 3.0
    } else {
        text_height
    };
    let total_rows = matching.len();

    // `stick_to_bottom` is egui's API for terminal/log follow. `animated(false)` keeps
    // follow updates from lerping through the buffer when content grows.
    egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .stick_to_bottom(stick_to_bottom)
        .animated(false)
        .auto_shrink([false, false])
        .show_rows(ui, row_height, total_rows, |ui, row_range| {
            for row in row_range {
                let line = &lines[matching[row]];
                let color = match style {
                    LogScrollStyle::ErrorsOnly => error_line_color(line.level),
                    LogScrollStyle::ByLevel => log_level_color(line.level),
                };
                let text = line.display(show_timestamps);
                if let Some(tag_filters) = tag_filters {
                    ui.label(build_highlight_job(ui, text, color, tag_filters));
                } else {
                    ui.colored_label(color, text);
                }
            }
        });
}

fn build_highlight_job(
    ui: &egui::Ui,
    text: &str,
    base_color: egui::Color32,
    tag_filters: &[LogcatTagFilter],
) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};

    let font_id = ui.style().text_styles[&egui::TextStyle::Monospace].clone();
    let default_format = TextFormat {
        font_id: font_id.clone(),
        color: base_color,
        ..Default::default()
    };

    if tag_filters.is_empty() {
        return LayoutJob::single_section(text.to_owned(), default_format);
    }

    #[derive(Clone, Copy)]
    struct HighlightRange {
        start: usize,
        end: usize,
        color_index: usize,
    }

    let text_lower = text.to_lowercase();
    let mut ranges = Vec::new();

    for filter in tag_filters {
        let needle = filter.tag.to_lowercase();
        if needle.is_empty() {
            continue;
        }

        let mut search_start = 0;
        while let Some(rel) = text_lower[search_start..].find(&needle) {
            let start = search_start + rel;
            let end = start + needle.len();
            ranges.push(HighlightRange {
                start,
                end,
                color_index: filter.color_index,
            });
            search_start = end;
        }
    }

    ranges.sort_by_key(|range| (range.start, -(range.end as isize - range.start as isize)));

    let mut merged = Vec::new();
    let mut last_end = 0;
    for range in ranges {
        if range.start >= last_end {
            merged.push(range);
            last_end = range.end;
        }
    }

    let mut job = LayoutJob::default();
    let mut pos = 0;
    if merged.is_empty() {
        job.append(text, 0.0, default_format);
        return job;
    }

    for range in merged {
        if pos < range.start {
            job.append(&text[pos..range.start], 0.0, default_format.clone());
        }

        let (background, foreground) =
            theme::colors::TAG_HIGHLIGHTS[range.color_index % theme::colors::TAG_HIGHLIGHTS.len()];
        job.append(
            &text[range.start..range.end],
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: foreground,
                background,
                ..Default::default()
            },
        );
        pos = range.end;
    }

    if pos < text.len() {
        job.append(&text[pos..], 0.0, default_format);
    }

    job
}

fn log_level_color(level: char) -> egui::Color32 {
    match level {
        'E' => theme::colors::LOG_ERROR,
        'W' => theme::colors::LOG_WARNING,
        'I' => theme::colors::LOG_INFO,
        'D' => theme::colors::LOG_DEBUG,
        'F' => theme::colors::LOG_FATAL,
        _ => theme::colors::LOG_DEFAULT,
    }
}

fn error_line_color(level: char) -> egui::Color32 {
    match level {
        'F' => theme::colors::LOG_FATAL,
        _ => theme::colors::LOG_ERROR,
    }
}
