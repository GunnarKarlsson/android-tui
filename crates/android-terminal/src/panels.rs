use std::collections::VecDeque;

use adb_client::SystemStats;
use eframe::egui;

use crate::app::{App, CachedLogLine};

pub fn logcat_all(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.auto_scroll, "Auto-scroll");
    });

    if let Some(error) = &app.logcat_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
    }

    if app.selected_serial.is_none() {
        ui.label("Select a device to start logcat.");
        return;
    }

    show_log_scroll(
        ui,
        &app.log_lines,
        app.auto_scroll,
        egui::Id::new("logcat_all_scroll"),
    );
}

pub fn logcat_errors(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        ui.label("Select a device to start logcat.");
        return;
    }

    show_log_scroll(
        ui,
        &app.error_lines,
        app.auto_scroll,
        egui::Id::new("logcat_errors_scroll"),
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
            show_system_stats(ui, stats);
        });
}

pub fn network(ui: &mut egui::Ui, app: &App) {
    if app.selected_serial.is_none() {
        ui.label("Select a device to view network activity.");
        return;
    }

    ui.label("Network stats — coming in Step 2.5");
}

fn show_system_stats(ui: &mut egui::Ui, stats: &SystemStats) {
    let memory = &stats.memory;

    ui.label(format!(
        "Memory: {} used / {} total ({} free, {} available)",
        format_kb(memory.used_kb()),
        format_kb(memory.total_kb),
        format_kb(memory.free_kb),
        format_kb(memory.available_kb),
    ));
    ui.label(format!(
        "Buffers: {}  Cached: {}",
        format_kb(memory.buffers_kb),
        format_kb(memory.cached_kb),
    ));

    let progress = memory.used_fraction();
    ui.add(
        egui::ProgressBar::new(progress)
            .text(format!("{:.0}% used", progress * 100.0))
            .fill(egui::Color32::from_rgb(90, 150, 220)),
    );

    ui.separator();
    ui.label("Disk");

    egui::ScrollArea::horizontal()
        .id_salt(egui::Id::new("disk_table_scroll"))
        .show(ui, |ui| {
            egui::Grid::new("disk_stats")
                .num_columns(6)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Filesystem");
                    ui.label("Size");
                    ui.label("Used");
                    ui.label("Avail");
                    ui.label("Use%");
                    ui.label("Mounted on");
                    ui.end_row();

                    for disk in &stats.disks {
                        ui.label(&disk.filesystem);
                        ui.label(&disk.size);
                        ui.label(&disk.used);
                        ui.label(&disk.available);
                        ui.label(&disk.use_percent);
                        ui.label(&disk.mount_point);
                        ui.end_row();
                    }
                });
        });
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
    auto_scroll: bool,
    scroll_id: egui::Id,
) {
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let total_rows = lines.len();

    egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .stick_to_bottom(auto_scroll)
        .auto_shrink([false, false])
        .show_rows(ui, row_height, total_rows, |ui, row_range| {
            for row in row_range {
                let line = &lines[row];
                ui.colored_label(log_level_color(line.level), &line.text);
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
