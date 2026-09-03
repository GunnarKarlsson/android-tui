use eframe::egui;
use egui_tiles::{Behavior, ResizeState, TileId, Tree, UiResponse};

use crate::app::App;
use crate::panels;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Devices,
    Ram,
    Storage,
    LogcatAll,
    LogcatErrors,
    Insight,
    SystemStats,
    Network,
    Protocols,
}

impl PanelId {
    fn title(self) -> &'static str {
        match self {
            PanelId::Devices => "Devices",
            PanelId::Ram => "RAM",
            PanelId::Storage => "Storage",
            PanelId::LogcatAll => "Logcat (All)",
            PanelId::LogcatErrors => "Logcat (Errors)",
            PanelId::Insight => "Insight",
            PanelId::SystemStats => "Storage Details",
            PanelId::Network => "Network Activity",
            PanelId::Protocols => "App Traffic",
        }
    }
}

pub fn create_default_tree() -> Tree<PanelId> {
    let mut tiles = egui_tiles::Tiles::default();

    let devices = tiles.insert_pane(PanelId::Devices);
    let ram = tiles.insert_pane(PanelId::Ram);
    let storage = tiles.insert_pane(PanelId::Storage);
    let left_column = tiles.insert_vertical_tile(vec![devices, ram, storage]);

    let logcat_all = tiles.insert_pane(PanelId::LogcatAll);
    let system_stats = tiles.insert_pane(PanelId::SystemStats);
    let middle_column = tiles.insert_vertical_tile(vec![logcat_all, system_stats]);

    let logcat_errors = tiles.insert_pane(PanelId::LogcatErrors);
    let insight = tiles.insert_pane(PanelId::Insight);
    let network = tiles.insert_pane(PanelId::Network);
    let protocols = tiles.insert_pane(PanelId::Protocols);
    let right_column = tiles.insert_vertical_tile(vec![logcat_errors, insight, network, protocols]);

    let root = tiles.insert_horizontal_tile(vec![left_column, middle_column, right_column]);

    set_linear_shares(
        &mut tiles,
        root,
        &[
            (left_column, 1.0),
            (middle_column, 2.5),
            (right_column, 2.5),
        ],
    );
    set_linear_shares(
        &mut tiles,
        left_column,
        &[(devices, 2.0), (ram, 1.5), (storage, 1.5)],
    );
    set_linear_shares(
        &mut tiles,
        middle_column,
        &[(logcat_all, 2.0), (system_stats, 1.0)],
    );
    set_linear_shares(
        &mut tiles,
        right_column,
        &[
            (logcat_errors, 2.0),
            (insight, 1.5),
            (network, 1.0),
            (protocols, 1.0),
        ],
    );

    Tree::new("android_terminal_tiles", root, tiles)
}

fn set_linear_shares(
    tiles: &mut egui_tiles::Tiles<PanelId>,
    container_id: egui_tiles::TileId,
    shares: &[(egui_tiles::TileId, f32)],
) {
    let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear))) =
        tiles.get_mut(container_id)
    else {
        return;
    };

    for (tile_id, share) in shares {
        linear.shares.set_share(*tile_id, *share);
    }
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

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        theme::PANEL_GAP
    }

    fn resize_stroke(&self, _style: &egui::Style, resize_state: ResizeState) -> egui::Stroke {
        match resize_state {
            ResizeState::Idle => egui::Stroke::NONE,
            ResizeState::Hovering | ResizeState::Dragging => {
                egui::Stroke::new(1.0, theme::colors::PANEL_SPLITTER_HOVER)
            }
        }
    }

    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile_id: TileId, pane: &mut PanelId) -> UiResponse {
        match pane {
            PanelId::Devices => {
                self.app.show_devices(ui);
            }
            PanelId::Ram => {
                panels::ram_gauge(ui, self.app);
            }
            PanelId::Storage => {
                panels::storage_gauge(ui, self.app);
            }
            PanelId::LogcatAll => {
                let mut show_timestamps = self.app.logcat_show_timestamps;
                let mut line_spacing = self.app.logcat_line_spacing;
                theme::panel_with_header_actions(
                    ui,
                    pane.title(),
                    |ui| {
                        logcat_header_toggles(ui, &mut show_timestamps, &mut line_spacing);
                    },
                    |ui| panels::logcat_all(ui, self.app),
                );
                self.app.logcat_show_timestamps = show_timestamps;
                self.app.logcat_line_spacing = line_spacing;
            }
            PanelId::Insight => {
                theme::panel(ui, pane.title(), |ui| {
                    panels::insight(ui, self.app);
                });
            }
            PanelId::LogcatErrors => {
                let mut show_timestamps = self.app.error_show_timestamps;
                let mut line_spacing = self.app.error_line_spacing;
                theme::panel_with_header_actions(
                    ui,
                    pane.title(),
                    |ui| {
                        logcat_header_toggles(ui, &mut show_timestamps, &mut line_spacing);
                    },
                    |ui| panels::logcat_errors(ui, self.app),
                );
                self.app.error_show_timestamps = show_timestamps;
                self.app.error_line_spacing = line_spacing;
            }
            PanelId::SystemStats => {
                theme::panel(ui, pane.title(), |ui| panels::storage_usage(ui, self.app));
            }
            PanelId::Network => {
                theme::panel(ui, pane.title(), |ui| panels::network(ui, self.app));
            }
            PanelId::Protocols => {
                theme::panel(ui, pane.title(), |ui| panels::protocols(ui, self.app));
            }
        }

        UiResponse::None
    }
}

fn logcat_header_toggles(ui: &mut egui::Ui, show_timestamps: &mut bool, line_spacing: &mut bool) {
    if theme::icon_toggle(ui, theme::icons::CLOCK, *show_timestamps).clicked() {
        *show_timestamps = !*show_timestamps;
    }
    if theme::icon_toggle(ui, theme::icons::LINE_SPACING, *line_spacing).clicked() {
        *line_spacing = !*line_spacing;
    }
}
