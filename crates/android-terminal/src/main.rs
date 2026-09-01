mod app;
mod panels;
mod tiles;

use adb_client::{Adb, DeviceInfo};
use egui_tiles::Tree;

use crate::app::App;
use crate::tiles::PanelId;

struct TerminalApp {
    inner: App,
    tile_tree: Tree<PanelId>,
}

impl TerminalApp {
    fn new(
        adb_error: Option<String>,
        devices: Vec<DeviceInfo>,
        list_error: Option<String>,
    ) -> Self {
        Self {
            inner: App::new(adb_error, devices, list_error),
            tile_tree: tiles::create_default_tree(),
        }
    }
}

impl Drop for TerminalApp {
    fn drop(&mut self) {
        self.inner.shutdown();
    }
}

impl eframe::App for TerminalApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.inner.update_panels(ctx);

        eframe::egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.heading("Android Terminal");
        });

        eframe::egui::SidePanel::left("devices")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                self.inner.show_devices(ui);
            });

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            if self.inner.adb_error.is_some() {
                ui.label("ADB is not available. See the devices panel for details.");
                return;
            }

            tiles::show(ui, &mut self.tile_tree, &mut self.inner);
        });
    }
}

fn main() -> eframe::Result<()> {
    let adb_error = Adb::check_available().err().map(|e| e.to_string());

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
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

            Ok(Box::new(TerminalApp::new(adb_error, devices, list_error)))
        }),
    )
}
