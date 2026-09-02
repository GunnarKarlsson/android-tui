use eframe::egui;
use egui_tiles::{Behavior, ResizeState, TileId, Tree, UiResponse};

use crate::app::App;
use crate::panels;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    LogcatAll,
    LogcatErrors,
    SystemStats,
    Network,
    Protocols,
}

impl PanelId {
    fn title(self) -> &'static str {
        match self {
            PanelId::LogcatAll => "Logcat (All)",
            PanelId::LogcatErrors => "Logcat (Errors)",
            PanelId::SystemStats => "Memory / Disk",
            PanelId::Network => "Network Activity",
            PanelId::Protocols => "App Traffic",
        }
    }
}

pub fn create_default_tree() -> Tree<PanelId> {
    let mut tiles = egui_tiles::Tiles::default();

    let logcat_all = tiles.insert_pane(PanelId::LogcatAll);
    let logcat_errors = tiles.insert_pane(PanelId::LogcatErrors);
    let system_stats = tiles.insert_pane(PanelId::SystemStats);
    let network = tiles.insert_pane(PanelId::Network);
    let protocols = tiles.insert_pane(PanelId::Protocols);

    let top_row = tiles.insert_horizontal_tile(vec![logcat_all, logcat_errors]);
    let network_column = tiles.insert_vertical_tile(vec![network, protocols]);
    let bottom_row = tiles.insert_horizontal_tile(vec![system_stats, network_column]);
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

    fn resize_stroke(&self, style: &egui::Style, resize_state: ResizeState) -> egui::Stroke {
        let color = theme::colors::PANEL_SPLITTER;
        let width = match resize_state {
            ResizeState::Idle => self.gap_width(style),
            ResizeState::Hovering => style.visuals.widgets.hovered.fg_stroke.width.max(1.0),
            ResizeState::Dragging => style.visuals.widgets.active.fg_stroke.width.max(2.0),
        };
        egui::Stroke::new(width, color)
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: TileId,
        pane: &mut PanelId,
    ) -> UiResponse {
        match pane {
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
                theme::panel(ui, pane.title(), |ui| {
                    panels::memory_disk(ui, self.app)
                });
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
