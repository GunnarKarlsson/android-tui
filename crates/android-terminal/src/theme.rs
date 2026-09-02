//! Central egui style and theme configuration.

use eframe::egui::{self, Context, Ui};

/// Apply app-wide egui styling. Called once at startup from the eframe creation hook.
pub fn configure(ctx: &Context) {
    let mut style = (*ctx.style()).clone();

    // Inset for tile panel bodies (see [`panel_content`]).
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);

    ctx.set_style(style);
}

/// Margin applied inside each tile panel, below the title separator.
fn panel_inner_margin(ui: &Ui) -> egui::Margin {
    // egui has no dedicated "tile pane" spacing; we reuse window_margin as the
    // single source of truth configured in [`configure`].
    ui.style().spacing.window_margin
}

/// Wrap panel body content with the themed inner margin.
pub fn panel_content<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE
        .inner_margin(panel_inner_margin(ui))
        .show(ui, add_contents)
        .inner
}
