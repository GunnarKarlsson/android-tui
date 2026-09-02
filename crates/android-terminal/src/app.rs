use std::collections::VecDeque;
use std::time::Duration;

use adb_client::{
    Adb, AppStoragePoller, AppStorageUpdate, DeviceInfo, DeviceState, LogEntry, LogcatStream,
    NetworkPoller, NetworkUpdate, ProtocolPoller, ProtocolStats, ProtocolUpdate, StatsPoller,
    StatsUpdate, SystemStats,
};
use crossbeam_channel::Receiver;
use eframe::egui;

use crate::panels;
use crate::theme;

pub const MAX_LOG_LINES: usize = 10_000;
const MAX_DRAIN_PER_FRAME: usize = 500;

pub struct App {
    pub adb_error: Option<String>,
    pub devices: Vec<DeviceInfo>,
    pub list_error: Option<String>,
    pub selected_serial: Option<String>,
    pub logcat_rx: Option<Receiver<LogEntry>>,
    pub logcat_stream: Option<LogcatStream>,
    pub error_logcat_rx: Option<Receiver<LogEntry>>,
    pub error_logcat_stream: Option<LogcatStream>,
    pub stats_rx: Option<Receiver<StatsUpdate>>,
    pub stats_poller: Option<StatsPoller>,
    pub network_rx: Option<Receiver<NetworkUpdate>>,
    pub network_poller: Option<NetworkPoller>,
    pub protocol_rx: Option<Receiver<ProtocolUpdate>>,
    pub protocol_poller: Option<ProtocolPoller>,
    pub system_stats: Option<SystemStats>,
    pub stats_error: Option<String>,
    pub network_stats: Option<Vec<panels::NetworkRow>>,
    pub network_error: Option<String>,
    pub protocol_stats: Option<ProtocolStats>,
    pub protocol_error: Option<String>,
    pub app_storage_rx: Option<Receiver<AppStorageUpdate>>,
    pub app_storage_poller: Option<AppStoragePoller>,
    pub app_storage: panels::AppStorageState,
    pub log_lines: VecDeque<CachedLogLine>,
    pub error_lines: VecDeque<CachedLogLine>,
    pub logcat_error: Option<String>,
    pub error_logcat_error: Option<String>,
    pub auto_update_feed: bool,
    pub error_auto_update_feed: bool,
    pub logcat_show_timestamps: bool,
    pub error_show_timestamps: bool,
    pub logcat_line_spacing: bool,
    pub error_line_spacing: bool,
    pub logcat_filter: String,
    pub error_logcat_filter: String,
}

#[derive(Clone)]
pub struct CachedLogLine {
    full: String,
    compact: String,
    pub level: char,
}

impl CachedLogLine {
    pub fn from_entry(entry: &LogEntry) -> Self {
        CachedLogLine {
            full: entry.format_line_with_timestamp(true),
            compact: entry.format_line_with_timestamp(false),
            level: entry.level,
        }
    }

    pub fn display(&self, show_timestamp: bool) -> &str {
        if show_timestamp {
            &self.full
        } else {
            &self.compact
        }
    }

    pub fn matches_filter(&self, filter_lower: &str) -> bool {
        self.full.to_lowercase().contains(filter_lower)
    }
}

impl App {
    pub fn new(adb_error: Option<String>, devices: Vec<DeviceInfo>, list_error: Option<String>) -> Self {
        let mut app = App {
            adb_error,
            devices,
            list_error,
            selected_serial: None,
            logcat_rx: None,
            logcat_stream: None,
            error_logcat_rx: None,
            error_logcat_stream: None,
            stats_rx: None,
            stats_poller: None,
            network_rx: None,
            network_poller: None,
            protocol_rx: None,
            protocol_poller: None,
            system_stats: None,
            stats_error: None,
            network_stats: None,
            network_error: None,
            protocol_stats: None,
            protocol_error: None,
            app_storage_rx: None,
            app_storage_poller: None,
            app_storage: panels::AppStorageState::default(),
            log_lines: VecDeque::new(),
            error_lines: VecDeque::new(),
            logcat_error: None,
            error_logcat_error: None,
            auto_update_feed: true,
            error_auto_update_feed: true,
            logcat_show_timestamps: true,
            error_show_timestamps: true,
            logcat_line_spacing: false,
            error_line_spacing: false,
            logcat_filter: String::new(),
            error_logcat_filter: String::new(),
        };
        if let Some(serial) = first_ready_serial(&app.devices) {
            app.select_device(serial);
        }
        app
    }

    pub fn refresh_devices(&mut self) {
        self.list_error = None;
        match Adb::list_devices() {
            Ok(devices) => {
                self.devices = devices;
                if let Some(serial) = &self.selected_serial {
                    let still_connected = self.devices.iter().any(|device| {
                        device.serial == *serial && device.state == DeviceState::Device
                    });
                    if !still_connected {
                        self.deselect_device();
                    }
                }
                if self.selected_serial.is_none() {
                    if let Some(serial) = first_ready_serial(&self.devices) {
                        self.select_device(serial);
                    }
                }
            }
            Err(err) => self.list_error = Some(err.user_message()),
        }
    }

    pub fn select_device(&mut self, serial: String) {
        if self.selected_serial.as_deref() == Some(serial.as_str()) {
            return;
        }

        self.stop_streams();
        self.clear_device_data();
        self.selected_serial = Some(serial.clone());
        self.start_streams(&serial);
    }

    pub fn deselect_device(&mut self) {
        if self.selected_serial.is_none() {
            return;
        }

        self.stop_streams();
        self.clear_device_data();
        self.selected_serial = None;
    }

    pub fn shutdown(&mut self) {
        self.stop_streams();
    }

    fn start_streams(&mut self, serial: &str) {
        match LogcatStream::spawn(serial) {
            Ok((rx, stream)) => {
                self.logcat_rx = Some(rx);
                self.logcat_stream = Some(stream);
            }
            Err(err) => self.logcat_error = Some(err.user_message()),
        }

        match LogcatStream::spawn_errors(serial) {
            Ok((rx, stream)) => {
                self.error_logcat_rx = Some(rx);
                self.error_logcat_stream = Some(stream);
            }
            Err(err) => self.error_logcat_error = Some(err.user_message()),
        }

        match StatsPoller::spawn(serial) {
            Ok((rx, poller)) => {
                self.stats_rx = Some(rx);
                self.stats_poller = Some(poller);
            }
            Err(err) => self.stats_error = Some(err.user_message()),
        }

        match NetworkPoller::spawn(serial) {
            Ok((rx, poller)) => {
                self.network_rx = Some(rx);
                self.network_poller = Some(poller);
            }
            Err(err) => self.network_error = Some(err.user_message()),
        }

        match ProtocolPoller::spawn(serial) {
            Ok((rx, poller)) => {
                self.protocol_rx = Some(rx);
                self.protocol_poller = Some(poller);
            }
            Err(err) => self.protocol_error = Some(err.user_message()),
        }

        // Heavy per-package scan; start after fast pollers and an internal delay.
        match AppStoragePoller::spawn(serial) {
            Ok((rx, poller)) => {
                self.app_storage_rx = Some(rx);
                self.app_storage_poller = Some(poller);
                self.app_storage.scanning = true;
            }
            Err(err) => self.app_storage.error = Some(err.user_message()),
        }
    }

    fn clear_device_data(&mut self) {
        self.log_lines.clear();
        self.error_lines.clear();
        self.logcat_error = None;
        self.error_logcat_error = None;
        self.logcat_filter.clear();
        self.error_logcat_filter.clear();
        self.system_stats = None;
        self.stats_error = None;
        self.network_stats = None;
        self.network_error = None;
        self.protocol_stats = None;
        self.protocol_error = None;
        self.app_storage = panels::AppStorageState::default();
    }

    fn stop_streams(&mut self) {
        self.stop_logcat();
        self.stop_stats();
        self.stop_network();
        self.stop_protocols();
        self.stop_app_storage();
    }

    fn stop_network(&mut self) {
        if let Some(poller) = self.network_poller.take() {
            poller.stop();
        }
        self.network_rx = None;
    }

    fn stop_protocols(&mut self) {
        if let Some(poller) = self.protocol_poller.take() {
            poller.stop();
        }
        self.protocol_rx = None;
    }

    fn stop_app_storage(&mut self) {
        if let Some(poller) = self.app_storage_poller.take() {
            poller.stop();
        }
        self.app_storage_rx = None;
    }

    fn stop_stats(&mut self) {
        if let Some(poller) = self.stats_poller.take() {
            poller.stop();
        }
        self.stats_rx = None;
    }

    fn stop_logcat(&mut self) {
        if let Some(stream) = self.logcat_stream.take() {
            stream.stop();
        }
        self.logcat_rx = None;
        if let Some(stream) = self.error_logcat_stream.take() {
            stream.stop();
        }
        self.error_logcat_rx = None;
    }

    fn drain_logcat(&mut self) -> bool {
        let entries = take_log_entries(self.logcat_rx.as_ref(), self.auto_update_feed);
        if entries.is_empty() {
            return false;
        }
        for entry in entries {
            self.log_lines.push_back(CachedLogLine::from_entry(&entry));
            trim_buffer(&mut self.log_lines);
        }
        true
    }

    fn drain_error_logcat(&mut self) -> bool {
        let entries = take_log_entries(self.error_logcat_rx.as_ref(), self.error_auto_update_feed);
        if entries.is_empty() {
            return false;
        }
        let mut updated = false;
        for entry in entries {
            if entry.is_error_level() {
                self.error_lines
                    .push_back(CachedLogLine::from_entry(&entry));
                trim_buffer(&mut self.error_lines);
                updated = true;
            }
        }
        updated
    }

    fn drain_stats(&mut self) -> bool {
        let Some(rx) = self.stats_rx.as_ref() else {
            return false;
        };

        let mut updated = false;
        while let Ok(update) = rx.try_recv() {
            match update {
                StatsUpdate::Stats(stats) => {
                    self.system_stats = Some(stats);
                    self.stats_error = None;
                    updated = true;
                }
                StatsUpdate::Error(message) => {
                    self.stats_error = Some(message);
                    updated = true;
                }
            }
        }
        updated
    }

    fn drain_network(&mut self) -> bool {
        let Some(rx) = self.network_rx.as_ref() else {
            return false;
        };

        let mut updated = false;
        while let Ok(update) = rx.try_recv() {
            match update {
                NetworkUpdate::Stats(stats) => {
                    self.network_stats = Some(panels::network_rows_from_stats(&stats));
                    self.network_error = None;
                    updated = true;
                }
                NetworkUpdate::Error(message) => {
                    self.network_error = Some(message);
                    updated = true;
                }
            }
        }
        updated
    }

    fn drain_protocols(&mut self) -> bool {
        let Some(rx) = self.protocol_rx.as_ref() else {
            return false;
        };

        let mut updated = false;
        while let Ok(update) = rx.try_recv() {
            match update {
                ProtocolUpdate::Stats(stats) => {
                    self.protocol_stats = Some(stats);
                    self.protocol_error = None;
                    updated = true;
                }
                ProtocolUpdate::Error(message) => {
                    self.protocol_error = Some(message);
                    updated = true;
                }
            }
        }
        updated
    }

    fn drain_app_storage(&mut self) -> bool {
        let Some(rx) = self.app_storage_rx.as_ref() else {
            return false;
        };

        let mut updated = false;
        while let Ok(update) = rx.try_recv() {
            match update {
                AppStorageUpdate::PackageList(packages) => {
                    if self.app_storage.packages.is_empty() {
                        self.app_storage.set_packages(packages);
                    } else {
                        self.app_storage.merge_packages(packages);
                    }
                    self.app_storage.error = None;
                    updated = true;
                }
                AppStorageUpdate::PackageStorage(storage) => {
                    self.app_storage
                        .set_size(&storage.package, storage.total_bytes);
                    updated = true;
                }
                AppStorageUpdate::ScanComplete => {
                    self.app_storage.scanning = false;
                    updated = true;
                }
                AppStorageUpdate::Error(message) => {
                    self.app_storage.error = Some(message);
                    updated = true;
                }
            }
        }
        updated
    }

    pub fn update_panels(&mut self, ctx: &egui::Context) {
        let mut needs_repaint = false;
        if self.drain_logcat() {
            needs_repaint = true;
        }
        if self.drain_error_logcat() {
            needs_repaint = true;
        }
        if self.drain_stats() {
            needs_repaint = true;
        }
        if self.drain_network() {
            needs_repaint = true;
        }
        if self.drain_protocols() {
            needs_repaint = true;
        }
        if self.drain_app_storage() {
            needs_repaint = true;
        }
        if needs_repaint {
            ctx.request_repaint();
        }
        if self.selected_serial.is_some() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    }

    pub fn show_devices(&mut self, ui: &mut egui::Ui) {
        let mut refresh = false;
        theme::panel_with_header_actions(
            ui,
            "Devices",
            |ui| {
                refresh = theme::icon_button(ui, theme::icons::REFRESH).clicked();
            },
            |ui| {
                self.show_devices_body(ui);
            },
        );
        if refresh {
            self.refresh_devices();
        }
    }

    fn show_devices_body(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = &self.adb_error {
            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), "ADB not available");
            ui.label(error);
            return;
        }

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
                            if selected {
                                self.deselect_device();
                            } else {
                                let serial = self.devices[index].serial.clone();
                                self.select_device(serial);
                            }
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

fn take_log_entries(rx: Option<&Receiver<LogEntry>>, auto_update: bool) -> Vec<LogEntry> {
    let Some(rx) = rx else {
        return Vec::new();
    };

    if !auto_update {
        while rx.try_recv().is_ok() {}
        return Vec::new();
    }

    rx.try_iter().take(MAX_DRAIN_PER_FRAME).collect()
}

fn trim_buffer(buffer: &mut VecDeque<CachedLogLine>) {
    while buffer.len() > MAX_LOG_LINES {
        buffer.pop_front();
    }
}

fn first_ready_serial(devices: &[DeviceInfo]) -> Option<String> {
    devices
        .iter()
        .find(|device| device.state == DeviceState::Device)
        .map(|device| device.serial.clone())
}

fn device_state_label(state: &DeviceState) -> &str {
    match state {
        DeviceState::Device => "device",
        DeviceState::Offline => "offline",
        DeviceState::Unauthorized => "unauthorized",
        DeviceState::Other(value) => value,
    }
}
