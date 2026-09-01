use std::collections::VecDeque;

use adb_client::DiskStats;
use eframe::egui;

use crate::app::{App, CachedLogLine};

pub fn logcat_all(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.auto_scroll, "Auto-scroll");
    });
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add(
            egui::TextEdit::singleline(&mut app.logcat_filter)
                .hint_text("Search logs…")
                .desired_width(ui.available_width()),
        )
        .on_hover_text("Filter log lines by text");
    });

    if let Some(error) = &app.logcat_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
    }

    if app.selected_serial.is_none() {
        ui.label("Select a device to start logcat.");
        return;
    }

    let filter = app.logcat_filter.clone();
    let matching = filtered_line_indices(&app.log_lines, &filter);
    ui.label(format!(
        "Showing {} of {} lines",
        matching.len(),
        app.log_lines.len()
    ));

    show_log_scroll(
        ui,
        &app.log_lines,
        &matching,
        app.auto_scroll,
        egui::Id::new("logcat_all_scroll"),
        LogScrollStyle::ByLevel,
    );
}

pub fn logcat_errors(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        ui.label("Select a device to start logcat.");
        return;
    }

    ui.label(format!("{} error lines", app.error_lines.len()));

    let matching: Vec<usize> = (0..app.error_lines.len()).collect();
    show_log_scroll(
        ui,
        &app.error_lines,
        &matching,
        app.auto_scroll,
        egui::Id::new("logcat_errors_scroll"),
        LogScrollStyle::ErrorsOnly,
    );
}

pub fn memory_disk(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        ui.label("Select a device to view memory and disk stats.");
        return;
    }

    if let Some(error) = &app.stats_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
    }

    let Some(stats) = &app.system_stats else {
        ui.label("Polling device stats...");
        return;
    };

    egui::ScrollArea::vertical()
        .id_salt(egui::Id::new("memory_disk_scroll"))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            show_memory_stats(ui, &stats.memory);
            ui.separator();
            show_disk_stats(ui, &stats.disks);
        });
}

pub fn network(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        ui.label("Select a device to view network activity.");
        return;
    }

    if let Some(error) = &app.network_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
    }

    let Some(stats) = &app.network_stats else {
        ui.label("Polling…");
        return;
    };

    show_network_table(ui, stats);
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

fn show_network_table(ui: &mut egui::Ui, stats: &[NetworkRow]) {
    egui::ScrollArea::horizontal()
        .id_salt(egui::Id::new("network_table_scroll"))
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

/// Placeholder row type until Step 2.5 wires real network stats.
#[derive(Clone)]
pub struct NetworkRow {
    pub interface: String,
    pub transport: String,
    pub rx: String,
    pub tx: String,
    pub rate: String,
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
        .filter(|(_, line)| line.text.to_lowercase().contains(&filter_lower))
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
    auto_scroll: bool,
    scroll_id: egui::Id,
    style: LogScrollStyle,
) {
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let total_rows = matching.len();

    egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .stick_to_bottom(auto_scroll)
        .auto_shrink([false, false])
        .show_rows(ui, row_height, total_rows, |ui, row_range| {
            for row in row_range {
                let line = &lines[matching[row]];
                let color = match style {
                    LogScrollStyle::ErrorsOnly => error_line_color(line.level),
                    LogScrollStyle::ByLevel => log_level_color(line.level),
                };
                ui.colored_label(color, &line.text);
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
