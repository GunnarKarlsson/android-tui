//! Central egui style and theme configuration.

use std::sync::Arc;

use eframe::egui::{
    self, Context, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Ui,
};

const JETBRAINS_MONO: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
const JETBRAINS_MONO_BOLD: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMonoNerdFont-Bold.ttf");

/// Apply app-wide egui styling. Called once at startup from the eframe creation hook.
pub fn configure(ctx: &Context) {
    install_fonts(ctx);

    let mut style = (*ctx.style()).clone();

    // Inset for tile panel bodies (see [`panel_content`]).
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);

    let bold = FontFamily::Name("jetbrains_mono_bold".into());
    style
        .text_styles
        .insert(TextStyle::Small, FontId::new(10.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::new(14.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::new(20.0, bold));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));

    ctx.set_style(style);
}

fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_MONO)),
    );
    fonts.font_data.insert(
        "jetbrains_mono_bold".to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_MONO_BOLD)),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "jetbrains_mono".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains_mono".to_owned());
    fonts.families.insert(
        FontFamily::Name("jetbrains_mono_bold".into()),
        vec!["jetbrains_mono_bold".to_owned()],
    );

    ctx.set_fonts(fonts);
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
