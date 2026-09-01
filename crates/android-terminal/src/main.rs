use adb_client::Adb;
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
        Box::new(|_cc| Ok(Box::new(App { adb_error }))),
    )
}

struct App {
    adb_error: Option<String>,
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
            } else {
                ui.separator();
                ui.label("ADB is available. Connect an emulator or USB device to get started.");
            }
        });
    }
}
