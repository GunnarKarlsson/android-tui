use std::collections::VecDeque;

use adb_client::{Adb, DeviceInfo, DeviceState, LogEntry, LogcatStream};
use crossbeam_channel::Receiver;
use eframe::egui;

const MAX_LOG_LINES: usize = 10_000;

fn main() -> eframe::Result<()> {
    let adb_error = Adb::check_available().err().map(|e| e.to_string());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
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
                log_lines: VecDeque::new(),
                logcat_error: None,
                auto_scroll: true,
            }))
        }),
    )
}

struct App {
    adb_error: Option<String>,
    devices: Vec<DeviceInfo>,
    list_error: Option<String>,
    selected_serial: Option<String>,
    logcat_rx: Option<Receiver<LogEntry>>,
    logcat_stream: Option<LogcatStream>,
    log_lines: VecDeque<LogEntry>,
    logcat_error: Option<String>,
    auto_scroll: bool,
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
        self.selected_serial = Some(serial.clone());
        self.log_lines.clear();
        self.logcat_error = None;

        match LogcatStream::spawn(&serial) {
            Ok((rx, stream)) => {
                self.logcat_rx = Some(rx);
                self.logcat_stream = Some(stream);
            }
            Err(err) => self.logcat_error = Some(err.to_string()),
        }
    }

    fn stop_logcat(&mut self) {
        self.logcat_rx = None;
        self.logcat_stream = None;
    }

    fn drain_logcat(&mut self) -> bool {
        let Some(rx) = self.logcat_rx.as_ref() else {
            return false;
        };

        let mut new_lines = false;
        while let Ok(entry) = rx.try_recv() {
            new_lines = true;
            self.log_lines.push_back(entry);
            while self.log_lines.len() > MAX_LOG_LINES {
                self.log_lines.pop_front();
            }
        }
        new_lines
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.drain_logcat() {
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

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for device in self.devices.clone() {
                        let selected =
                            self.selected_serial.as_deref() == Some(device.serial.as_str());
                        let label = format!("{}\n{}", device.model, device.serial);

                        if device.state == DeviceState::Device {
                            if ui.selectable_label(selected, label).clicked() {
                                self.select_device(device.serial);
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

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.adb_error.is_some() {
                return;
            }

            ui.horizontal(|ui| {
                ui.label("Logcat");
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
            });

            if let Some(error) = &self.logcat_error {
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), error);
            }

            if self.selected_serial.is_none() {
                ui.label("Select a device to start logcat.");
                return;
            }

            egui::ScrollArea::vertical()
                .stick_to_bottom(self.auto_scroll)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                    for entry in &self.log_lines {
                        ui.colored_label(log_level_color(entry.level), entry.format_line());
                    }
                });
        });
    }
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
