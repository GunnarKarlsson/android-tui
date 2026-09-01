use adb_client::{Adb, DeviceInfo, DeviceState};
use eframe::egui;

fn main() -> eframe::Result<()> {
    let adb_error = Adb::check_available().err().map(|e| e.to_string());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
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
            }))
        }),
    )
}

struct App {
    adb_error: Option<String>,
    devices: Vec<DeviceInfo>,
    list_error: Option<String>,
}

impl App {
    fn refresh_devices(&mut self) {
        self.list_error = None;
        match Adb::list_devices() {
            Ok(devices) => self.devices = devices,
            Err(err) => self.list_error = Some(err.to_string()),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Android Terminal");

            if let Some(error) = &self.adb_error {
                ui.separator();
                ui.colored_label(egui::Color32::from_rgb(220, 80, 80), "ADB not available");
                ui.label(error);
                ui.add_space(8.0);
                ui.label("Prerequisites:");
                ui.label("• Install Android SDK platform-tools");
                ui.label("• Ensure `adb` is on your PATH (`adb version` should work in a terminal)");
                return;
            }

            ui.separator();
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
                ui.label("No devices found. Connect an emulator or USB device.");
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for device in &self.devices {
                    ui.group(|ui| {
                        ui.label(format!("Model: {}", device.model));
                        ui.label(format!("Serial: {}", device.serial));
                        ui.label(format!("State: {}", device_state_label(&device.state)));
                    });
                    ui.add_space(4.0);
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
