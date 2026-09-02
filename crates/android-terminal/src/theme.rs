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

    // Single padding used by every panel (header, body, all four sides).
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

/// Outer frame for SidePanel / CentralPanel / TopBottomPanel.
/// Zero inner margin so [`panel`] is the only source of padding.
pub fn shell_frame(ctx: &Context) -> egui::Frame {
    egui::Frame::NONE.fill(ctx.style().visuals.panel_fill)
}

fn panel_padding(ui: &Ui) -> egui::Margin {
    ui.style().spacing.window_margin
}

/// Nerd Font glyphs from JetBrains Mono (see `assets/fonts`).
pub mod icons {
    /// Circular arrows — `nf-md-refresh`.
    pub const REFRESH: &str = "\u{f0450}";
}

/// Icon-only button.
pub fn icon_button(ui: &mut Ui, icon: &str) -> egui::Response {
    ui.button(icon)
}

/// Space below a toolbar row (filter, etc.), matching panel padding.
pub fn section_gap(ui: &mut Ui) {
    ui.add_space(panel_padding(ui).bottom as f32);
}

/// Filter row with themed spacing underneath.
pub fn filter_row(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    ui.horizontal(add_contents);
    section_gap(ui);
}
pub fn panel_loading(ui: &mut Ui) {
    ui.label("Loading...");
}

/// App title strip: same padding as [`panel`], no body.
pub fn title_bar(ui: &mut Ui, title: impl Into<egui::RichText>) {
    egui::Frame::NONE
        .inner_margin(panel_padding(ui))
        .show(ui, |ui| {
            ui.heading(title);
        });
}

/// One chrome for every panel: uniform padding on all four sides, title, then body.
pub fn panel<R>(
    ui: &mut Ui,
    title: impl Into<egui::RichText>,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> R {
    panel_with_header_actions(ui, title, |_| {}, add_body)
}

/// Like [`panel`], with extra widgets on the header row (e.g. the devices refresh icon).
pub fn panel_with_header_actions<R>(
    ui: &mut Ui,
    title: impl Into<egui::RichText>,
    add_header_actions: impl FnOnce(&mut Ui),
    add_body: impl FnOnce(&mut Ui) -> R,
) -> R {
    egui::Frame::NONE
        .inner_margin(panel_padding(ui))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(title);
                add_header_actions(ui);
            });
            ui.separator();
            add_body(ui)
        })
        .inner
}
