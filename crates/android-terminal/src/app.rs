use std::collections::VecDeque;

use adb_client::{
    Adb, DeviceInfo, DeviceState, LogEntry, LogcatStream, StatsPoller, SystemStats,
};
use crossbeam_channel::Receiver;
use eframe::egui;

pub const MAX_LOG_LINES: usize = 10_000;
const MAX_DRAIN_PER_FRAME: usize = 500;

pub struct App {
    pub adb_error: Option<String>,
    pub devices: Vec<DeviceInfo>,
    pub list_error: Option<String>,
    pub selected_serial: Option<String>,
    pub logcat_rx: Option<Receiver<LogEntry>>,
    pub logcat_stream: Option<LogcatStream>,
    pub stats_rx: Option<Receiver<SystemStats>>,
    pub stats_poller: Option<StatsPoller>,
    pub system_stats: Option<SystemStats>,
    pub stats_error: Option<String>,
    pub log_lines: VecDeque<CachedLogLine>,
    pub error_lines: VecDeque<CachedLogLine>,
    pub logcat_error: Option<String>,
    pub auto_scroll: bool,
}

#[derive(Clone)]
pub struct CachedLogLine {
    pub text: String,
    pub level: char,
}

impl CachedLogLine {
    pub fn from_entry(entry: &LogEntry) -> Self {
        CachedLogLine {
            text: entry.format_line(),
            level: entry.level,
        }
    }
}

impl App {
    pub fn new(adb_error: Option<String>, devices: Vec<DeviceInfo>, list_error: Option<String>) -> Self {
        App {
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
        }
    }

    pub fn refresh_devices(&mut self) {
        self.list_error = None;
        match Adb::list_devices() {
            Ok(devices) => self.devices = devices,
            Err(err) => self.list_error = Some(err.to_string()),
        }
    }

    pub fn select_device(&mut self, serial: String) {
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

    pub fn update_panels(&mut self, ctx: &egui::Context) {
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
    }

    pub fn show_devices(&mut self, ui: &mut egui::Ui) {
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
    }
}

fn trim_buffer(buffer: &mut VecDeque<CachedLogLine>) {
    while buffer.len() > MAX_LOG_LINES {
        buffer.pop_front();
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
