use eframe::egui;
use egui_tiles::{Behavior, TileId, Tree, UiResponse};

use crate::app::App;
use crate::panels;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    LogcatAll,
    LogcatErrors,
    SystemStats,
    Network,
}

impl PanelId {
    fn title(self) -> &'static str {
        match self {
            PanelId::LogcatAll => "Logcat (All)",
            PanelId::LogcatErrors => "Logcat (Errors)",
            PanelId::SystemStats => "Memory / Disk",
            PanelId::Network => "Network Activity",
        }
    }
}

pub fn create_default_tree() -> Tree<PanelId> {
    let mut tiles = egui_tiles::Tiles::default();

    let logcat_all = tiles.insert_pane(PanelId::LogcatAll);
    let logcat_errors = tiles.insert_pane(PanelId::LogcatErrors);
    let system_stats = tiles.insert_pane(PanelId::SystemStats);
    let network = tiles.insert_pane(PanelId::Network);

    let top_row = tiles.insert_horizontal_tile(vec![logcat_all, logcat_errors]);
    let bottom_row = tiles.insert_horizontal_tile(vec![system_stats, network]);
    let root = tiles.insert_vertical_tile(vec![top_row, bottom_row]);

    Tree::new("android_terminal_tiles", root, tiles)
}

pub fn show(ui: &mut egui::Ui, tree: &mut Tree<PanelId>, app: &mut App) {
    let mut behavior = AppTilesBehavior { app };
    tree.ui(&mut behavior, ui);
}

struct AppTilesBehavior<'a> {
    app: &'a mut App,
}

impl Behavior<PanelId> for AppTilesBehavior<'_> {
    fn tab_title_for_pane(&mut self, pane: &PanelId) -> egui::WidgetText {
        pane.title().into()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: TileId,
        pane: &mut PanelId,
    ) -> UiResponse {
        theme::panel_header(ui, pane.title());
        ui.separator();

        theme::panel_content(ui, |ui| match pane {
            PanelId::LogcatAll => panels::logcat_all(ui, self.app),
            PanelId::LogcatErrors => panels::logcat_errors(ui, self.app),
            PanelId::SystemStats => panels::memory_disk(ui, self.app),
            PanelId::Network => panels::network(ui, self.app),
        });

        UiResponse::None
    }
}
