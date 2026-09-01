use std::collections::VecDeque;

use adb_client::{
    Adb, DeviceInfo, DeviceState, LogEntry, LogcatStream, StatsPoller, SystemStats,
};
use crossbeam_channel::Receiver;
use eframe::egui;

const MAX_LOG_LINES: usize = 10_000;
const MAX_DRAIN_PER_FRAME: usize = 500;
const DEFAULT_STATS_PANEL_HEIGHT: f32 = 140.0;
const MIN_STATS_PANEL_HEIGHT: f32 = 96.0;
const MIN_LOGCAT_PANEL_HEIGHT: f32 = 120.0;
const STATS_RESIZE_HANDLE_HEIGHT: f32 = 10.0;

fn main() -> eframe::Result<()> {
    let adb_error = Adb::check_available().err().map(|e| e.to_string());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_title("Android Terminal"),
        ..Default::default()
    };

    eframe::run_native(
        "Android Terminal",
        options,
        Box::new(|_cc| {
            let (devices, list_error) = if adb_error.is_none() {
                match Adb::list_devices() {
                    Ok(devices) => (devices, None),
                    Err(err) => (Vec::new(), Some(err.to_string())),
                }
            } else {
                (Vec::new(), None)
            };

            Ok(Box::new(App {
                adb_error,
                devices,
                list_error,
                selected_serial: None,
                logcat_rx: None,
                logcat_stream: None,
                stats_rx: None,
                stats_poller: None,
                system_stats: None,
                stats_error: None,
                log_lines: VecDeque::new(),
                error_lines: VecDeque::new(),
                logcat_error: None,
                auto_scroll: true,
                stats_panel_height: DEFAULT_STATS_PANEL_HEIGHT,
            }))
        }),
    )
}

#[derive(Clone)]
struct CachedLogLine {
    text: String,
    level: char,
}

impl CachedLogLine {
    fn from_entry(entry: &LogEntry) -> Self {
        CachedLogLine {
            text: entry.format_line(),
            level: entry.level,
        }
    }
}

struct App {
    adb_error: Option<String>,
    devices: Vec<DeviceInfo>,
    list_error: Option<String>,
    selected_serial: Option<String>,
    logcat_rx: Option<Receiver<LogEntry>>,
    logcat_stream: Option<LogcatStream>,
    stats_rx: Option<Receiver<SystemStats>>,
    stats_poller: Option<StatsPoller>,
    system_stats: Option<SystemStats>,
    stats_error: Option<String>,
    log_lines: VecDeque<CachedLogLine>,
    error_lines: VecDeque<CachedLogLine>,
    logcat_error: Option<String>,
    auto_scroll: bool,
    stats_panel_height: f32,
}

impl App {
    fn refresh_devices(&mut self) {
        self.list_error = None;
        match Adb::list_devices() {
            Ok(devices) => self.devices = devices,
            Err(err) => self.list_error = Some(err.to_string()),
        }
    }

    fn select_device(&mut self, serial: String) {
        if self.selected_serial.as_deref() == Some(serial.as_str()) {
            return;
        }

        self.stop_logcat();
        self.stop_stats();
        self.selected_serial = Some(serial.clone());
        self.log_lines.clear();
        self.error_lines.clear();
        self.logcat_error = None;
        self.system_stats = None;
        self.stats_error = None;

        match LogcatStream::spawn(&serial) {
            Ok((rx, stream)) => {
                self.logcat_rx = Some(rx);
                self.logcat_stream = Some(stream);
            }
            Err(err) => self.logcat_error = Some(err.to_string()),
        }

        match StatsPoller::spawn(&serial) {
            Ok((rx, poller)) => {
                self.stats_rx = Some(rx);
                self.stats_poller = Some(poller);
            }
            Err(err) => self.stats_error = Some(err.to_string()),
        }
    }

    fn stop_stats(&mut self) {
        self.stats_rx = None;
        self.stats_poller = None;
    }

    fn stop_logcat(&mut self) {
        self.logcat_rx = None;
        self.logcat_stream = None;
    }

    fn push_line(&mut self, entry: LogEntry) {
        let cached = CachedLogLine::from_entry(&entry);
        if entry.is_error_level() {
            self.error_lines.push_back(cached.clone());
            trim_buffer(&mut self.error_lines);
        }
        self.log_lines.push_back(cached);
        trim_buffer(&mut self.log_lines);
    }

    fn drain_logcat(&mut self) -> bool {
        let entries: Vec<LogEntry> = match self.logcat_rx.as_ref() {
            Some(rx) => rx.try_iter().take(MAX_DRAIN_PER_FRAME).collect(),
            None => return false,
        };

        if entries.is_empty() {
            return false;
        }

        for entry in entries {
            self.push_line(entry);
        }
        true
    }

    fn drain_stats(&mut self) -> bool {
        let Some(rx) = self.stats_rx.as_ref() else {
            return false;
        };

        let mut updated = false;
        while let Ok(stats) = rx.try_recv() {
            self.system_stats = Some(stats);
            updated = true;
        }
        updated
    }
}

fn trim_buffer(buffer: &mut VecDeque<CachedLogLine>) {
    while buffer.len() > MAX_LOG_LINES {
        buffer.pop_front();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut needs_repaint = false;
        if self.drain_logcat() {
            needs_repaint = true;
        }
        if self.drain_stats() {
            needs_repaint = true;
        }
        if needs_repaint {
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.heading("Android Terminal");
        });

        egui::SidePanel::left("devices")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                if let Some(error) = &self.adb_error {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), "ADB not available");
                    ui.label(error);
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label("Devices");
                    if ui.button("Refresh").clicked() {
                        self.refresh_devices();
                    }
                });

                if let Some(error) = &self.list_error {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
                }

                if self.devices.is_empty() {
                    ui.label("No devices found.");
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_salt(egui::Id::new("device_list"))
                    .show(ui, |ui| {
                    let device_count = self.devices.len();
                    for index in 0..device_count {
                        let device = &self.devices[index];
                        let selected =
                            self.selected_serial.as_deref() == Some(device.serial.as_str());
                        let label = format!("{}\n{}", device.model, device.serial);

                        if device.state == DeviceState::Device {
                            if ui.selectable_label(selected, label).clicked() {
                                let serial = self.devices[index].serial.clone();
                                self.select_device(serial);
                            }
                        } else {
                            ui.add_enabled_ui(false, |ui| {
                                ui.label(format!(
                                    "{}\n{} ({})",
                                    device.model,
                                    device.serial,
                                    device_state_label(&device.state)
                                ));
                            });
                        }
                    }
                });
            });

        egui::SidePanel::right("logcat_errors")
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                bordered_panel(ui, "Logcat (Errors)", |ui| {
                    if self.adb_error.is_some() {
                        return;
                    }

                    if self.selected_serial.is_none() {
                        ui.label("Select a device to start logcat.");
                        return;
                    }

                    show_log_scroll(
                        ui,
                        &self.error_lines,
                        self.auto_scroll,
                        egui::Id::new("logcat_errors_scroll"),
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.adb_error.is_some() {
                return;
            }

            ui.vertical(|ui| {
                let total_height = ui.available_height();
                let max_stats_height =
                    (total_height - MIN_LOGCAT_PANEL_HEIGHT - STATS_RESIZE_HANDLE_HEIGHT).max(0.0);
                self.stats_panel_height = self
                    .stats_panel_height
                    .clamp(MIN_STATS_PANEL_HEIGHT, max_stats_height);
                let stats_height = self.stats_panel_height;
                let logcat_height =
                    (total_height - stats_height - STATS_RESIZE_HANDLE_HEIGHT).max(0.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), logcat_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_height(logcat_height);
                        bordered_panel(ui, "Logcat (All)", |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                            });

                            if let Some(error) = &self.logcat_error {
                                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
                            }

                            if self.selected_serial.is_none() {
                                ui.label("Select a device to start logcat.");
                                return;
                            }

                            show_log_scroll(
                                ui,
                                &self.log_lines,
                                self.auto_scroll,
                                egui::Id::new("logcat_all_scroll"),
                            );
                        });
                    },
                );

                let resize_response =
                    vertical_resize_handle(ui, STATS_RESIZE_HANDLE_HEIGHT, max_stats_height);
                if resize_response.dragged() {
                    self.stats_panel_height += resize_response.drag_delta().y;
                    self.stats_panel_height = self
                        .stats_panel_height
                        .clamp(MIN_STATS_PANEL_HEIGHT, max_stats_height);
                }

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), stats_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_height(stats_height);
                        bordered_panel(ui, "Memory / Disk", |ui| {
                            if self.selected_serial.is_none() {
                                ui.label("Select a device to view memory and disk stats.");
                                return;
                            }

                            if let Some(error) = &self.stats_error {
                                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
                            }

                            let Some(stats) = &self.system_stats else {
                                ui.label("Polling device stats...");
                                return;
                            };

                            egui::ScrollArea::vertical()
                                .id_salt(egui::Id::new("memory_disk_scroll"))
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    show_system_stats(ui, stats);
                                });
                        });
                    },
                );
            });
        });
    }
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

fn bordered_panel<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.heading(title);
            ui.separator();
            add_contents(ui)
        })
        .inner
}

fn vertical_resize_handle(
    ui: &mut egui::Ui,
    height: f32,
    _max_stats_height: f32,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::drag());

    if ui.is_rect_visible(rect) {
        let color = if response.hovered() || response.dragged() {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke.color
        };
        let y = rect.center().y;
        ui.painter()
            .hline(rect.x_range(), y, egui::Stroke::new(1.0, color));
    }

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    response
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
    let scroll_height = ui.available_height().max(row_height);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), scroll_height),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
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
        },
    );
}

fn device_state_label(state: &DeviceState) -> &str {
    match state {
        DeviceState::Device => "device",
        DeviceState::Offline => "offline",
        DeviceState::Unauthorized => "unauthorized",
        DeviceState::Other(value) => value,
    }
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
