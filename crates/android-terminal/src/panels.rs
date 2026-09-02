use std::collections::{HashMap, VecDeque};

use adb_client::{DiskStats, NetworkStats, ProtocolStats};
use eframe::egui;

use crate::app::{App, CachedLogLine};
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

pub fn logcat_all(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.auto_update_feed, "Auto-update feed");
    });
    theme::filter_row(ui, |ui| {
        ui.label("Filter:");
        ui.add(
            egui::TextEdit::singleline(&mut app.logcat_filter)
                .hint_text("Search logs…")
                .desired_width(ui.available_width()),
        )
        .on_hover_text("Filter log lines by text");
    });

    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if let Some(error) = &app.logcat_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
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
    let matching = filtered_line_indices(&app.log_lines, &filter);

    show_log_scroll(
        ui,
        &app.log_lines,
        &matching,
        app.auto_update_feed,
        app.logcat_show_timestamps,
        app.logcat_line_spacing,
        egui::Id::new("logcat_all_scroll"),
        LogScrollStyle::ByLevel,
    );
}

pub fn logcat_errors(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.error_auto_update_feed, "Auto-update feed");
    });
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
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
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
    let matching = filtered_line_indices(&app.error_lines, &filter);

    show_log_scroll(
        ui,
        &app.error_lines,
        &matching,
        app.error_auto_update_feed,
        app.error_show_timestamps,
        app.error_line_spacing,
        egui::Id::new("logcat_errors_scroll"),
        LogScrollStyle::ErrorsOnly,
    );
}

pub fn memory_disk(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if let Some(error) = &app.stats_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
    }
    if let Some(error) = &app.app_storage.error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
    }

    egui::ScrollArea::vertical()
        .id_salt(egui::Id::new("memory_disk_scroll"))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(stats) = &app.system_stats {
                show_memory_stats(ui, &stats.memory);
            } else if app.stats_rx.is_some() {
                ui.label("Loading memory…");
            }
            ui.separator();
            if let Some(stats) = &app.system_stats {
                show_disk_stats(ui, &stats.disks);
            } else if app.stats_rx.is_some() {
                ui.label("Loading disk…");
            }
            ui.separator();
            show_app_storage(ui, &app.app_storage);
        });
}

pub fn network(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if let Some(error) = &app.network_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
    }

    let Some(stats) = &app.network_stats else {
        if app.network_rx.is_some() {
            ui.label("Fetching network stats…");
        } else {
            theme::panel_loading(ui);
        }
        return;
    };

    if stats.is_empty() {
        ui.label("No network interfaces reported.");
        return;
    }

    show_network_table(ui, stats);
}

pub fn protocols(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        theme::panel_loading(ui);
        return;
    }

    if let Some(error) = &app.protocol_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
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
                        ui.label(format_bytes(app.total_bytes));
                        ui.label(format_bytes(app.foreground_bytes));
                        ui.label(format_bytes(app.background_bytes));
                        ui.label(format_bytes(app.wifi_bytes));
                        ui.label(format_bytes(app.mobile_bytes));
                        ui.end_row();
                    }
                });
        });
}

fn show_memory_stats(ui: &mut egui::Ui, memory: &adb_client::MemoryStats) {
    ui.label("Memory");
    ui.label(format!(
        "{} used / {} total",
        format_kb(memory.used_kb()),
        format_kb(memory.total_kb),
    ));
    ui.label(format!(
        "{} free, {} available",
        format_kb(memory.free_kb),
        format_kb(memory.available_kb),
    ));
    ui.label(format!(
        "Buffers: {}  Cached: {}",
        format_kb(memory.buffers_kb),
        format_kb(memory.cached_kb),
    ));

    let used_fraction = memory.used_fraction();
    ui.add(styled_progress_bar(
        used_fraction,
        format!("{:.0}% used", used_fraction * 100.0),
        egui::Color32::from_rgb(160, 200, 240),
    ));
}

fn show_disk_stats(ui: &mut egui::Ui, disks: &[DiskStats]) {
    ui.label("Disk");

    if disks.is_empty() {
        ui.label("No filesystems reported.");
        return;
    }

    for disk in disks {
        ui.horizontal(|ui| {
            ui.label(&disk.mount_point);
            ui.label(format!("{} / {}", disk.used, disk.size));
        });

        let fraction = parse_use_fraction(&disk.use_percent);
        ui.add(styled_progress_bar(
            fraction,
            format!("{} used — {}", disk.use_percent, disk.filesystem),
            disk_bar_color(fraction),
        ));
        ui.add_space(4.0);
    }
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

    egui::ScrollArea::vertical()
        .id_salt(egui::Id::new("app_storage_scroll"))
        .max_height(240.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("app_storage")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Package");
                    ui.label("Storage");
                    ui.end_row();

                    for (package, bytes) in storage.sorted_rows() {
                        ui.label(package);
                        ui.label(match bytes {
                            Some(value) => format_bytes(value),
                            None => "…".to_string(),
                        });
                        ui.end_row();
                    }
                });
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
            rx: format_bytes(iface.rx_bytes),
            tx: format_bytes(iface.tx_bytes),
            rate: format_rate(iface.rx_rate_bps, iface.tx_rate_bps),
        })
        .collect()
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

fn format_rate(rx_bps: f64, tx_bps: f64) -> String {
    format!("↓ {}  ↑ {}", format_throughput(rx_bps), format_throughput(tx_bps))
}

fn format_throughput(bps: f64) -> String {
    if bps < 0.0 {
        return "—".to_string();
    }
    if bps >= 1_048_576.0 {
        format!("{:.1} MB/s", bps / 1_048_576.0)
    } else if bps >= 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.0} B/s", bps)
    }
}

enum LogScrollStyle {
    ByLevel,
    ErrorsOnly,
}

fn filtered_line_indices(lines: &VecDeque<CachedLogLine>, filter: &str) -> Vec<usize> {
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

fn parse_use_fraction(use_percent: &str) -> f32 {
    use_percent
        .trim()
        .trim_end_matches('%')
        .parse::<f32>()
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

fn disk_bar_color(fraction: f32) -> egui::Color32 {
    if fraction >= 0.9 {
        egui::Color32::from_rgb(240, 170, 170)
    } else if fraction >= 0.75 {
        egui::Color32::from_rgb(240, 220, 150)
    } else {
        egui::Color32::from_rgb(160, 200, 240)
    }
}

fn styled_progress_bar(
    fraction: f32,
    text: impl Into<egui::WidgetText>,
    fill: egui::Color32,
) -> egui::ProgressBar {
    egui::ProgressBar::new(fraction)
        .text(text)
        .fill(fill)
        .corner_radius(egui::CornerRadius::ZERO)
}

fn format_kb(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1} GB", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{:.1} MB", kb as f64 / 1024.0)
    } else {
        format!("{kb} kB")
    }
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
) {
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
    let text_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let row_height = if line_spacing {
        text_height * 3.0
    } else {
        text_height
    };
    let total_rows = matching.len();

    egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .stick_to_bottom(stick_to_bottom)
        .auto_shrink([false, false])
        .show_rows(ui, row_height, total_rows, |ui, row_range| {
            for row in row_range {
                let line = &lines[matching[row]];
                let color = match style {
                    LogScrollStyle::ErrorsOnly => error_line_color(line.level),
                    LogScrollStyle::ByLevel => log_level_color(line.level),
                };
                let text = line.display(show_timestamps);
                if line_spacing {
                    ui.colored_label(color, format!("{text}\n\n"));
                } else {
                    ui.colored_label(color, text);
                }
            }
        });
}

fn log_level_color(level: char) -> egui::Color32 {
    match level {
        'E' => egui::Color32::from_rgb(220, 80, 80),
        'W' => egui::Color32::from_rgb(220, 180, 60),
        'I' => egui::Color32::from_rgb(120, 180, 255),
        'D' => egui::Color32::from_rgb(140, 140, 140),
        'F' => egui::Color32::from_rgb(255, 60, 60),
        _ => egui::Color32::GRAY,
    }
}

fn error_line_color(level: char) -> egui::Color32 {
    match level {
        'F' => egui::Color32::from_rgb(255, 60, 60),
        _ => egui::Color32::from_rgb(220, 80, 80),
    }
}
