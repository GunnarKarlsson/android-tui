use eframe::egui;

use crate::app::{App, InsightStatus};
use crate::theme;
use crate::ui_elements;

pub fn insight_panel(ui: &mut egui::Ui, app: &mut App, auto_scroll: bool) {
    if app.selected_serial.is_none() {
        ui_elements::panel_loading(ui);
        return;
    }

    ui_elements::panel_body(ui, theme::colors::INSIGHT_BODY, |ui| {
        if app.insight.replies.is_empty() {
            match app.insight.status {
                InsightStatus::RequestFailed => {
                    ui_elements::error_label(ui, "request failed, see log");
                }
                InsightStatus::Idle | InsightStatus::RequestSent => {
                    ui.label("...");
                }
            }
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt(egui::Id::new("insight_scroll"))
            .stick_to_bottom(auto_scroll)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for (index, reply) in app.insight.replies.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }
                    ui.label(reply.as_str());
                }
            });
    });
}
