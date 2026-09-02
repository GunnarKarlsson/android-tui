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

    ctx.all_styles_mut(apply_shared_style);
    ctx.set_visuals_of(egui::Theme::Dark, dark_blue_visuals());
    ctx.set_theme(egui::Theme::Dark);
}

fn apply_shared_style(style: &mut egui::Style) {
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
}

fn dark_blue_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();

    let bg = egui::Color32::from_rgb(10, 24, 52);
    let bg_widget = egui::Color32::from_rgb(18, 40, 78);
    let bg_hover = egui::Color32::from_rgb(28, 56, 104);
    let bg_extreme = egui::Color32::from_rgb(6, 16, 36);
    let bg_faint = egui::Color32::from_rgb(16, 36, 72);
    let white = egui::Color32::WHITE;

    visuals.panel_fill = bg;
    visuals.window_fill = bg;
    visuals.extreme_bg_color = bg_extreme;
    visuals.faint_bg_color = bg_faint;
    visuals.code_bg_color = bg_extreme;
    visuals.selection.bg_fill = egui::Color32::from_rgb(40, 80, 160);

    visuals.widgets.noninteractive.bg_fill = bg;
    visuals.widgets.noninteractive.weak_bg_fill = bg;
    visuals.widgets.noninteractive.fg_stroke.color = white;

    visuals.widgets.inactive.bg_fill = bg_widget;
    visuals.widgets.inactive.weak_bg_fill = bg_widget;
    visuals.widgets.inactive.fg_stroke.color = white;

    visuals.widgets.hovered.bg_fill = bg_hover;
    visuals.widgets.hovered.weak_bg_fill = bg_hover;
    visuals.widgets.hovered.fg_stroke.color = white;

    visuals.widgets.active.bg_fill = bg_hover;
    visuals.widgets.active.weak_bg_fill = bg_hover;
    visuals.widgets.active.fg_stroke.color = white;

    visuals.widgets.open.bg_fill = bg_widget;
    visuals.widgets.open.weak_bg_fill = bg_widget;
    visuals.widgets.open.fg_stroke.color = white;

    visuals
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

/// Outer frame for SidePanel / CentralPanel.
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
    /// Clock — `nf-md-clock`.
    pub const CLOCK: &str = "\u{f0954}";
    /// Two horizontal bars — `nf-md-view-headline`.
    pub const LINE_SPACING: &str = "\u{f0571}";
}

const ICON_BUTTON_PADDING: f32 = 6.0;

fn icon_button_widget(ui: &mut Ui, icon: &str, pressed: bool) -> egui::Response {
    let galley = egui::WidgetText::from(icon).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let ink = if galley.mesh_bounds.is_positive() {
        galley.mesh_bounds
    } else {
        galley.rect
    };
    let inner = ink.size().x.max(ink.size().y);
    let button_size = inner + 2.0 * ICON_BUTTON_PADDING;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(button_size, button_size),
        egui::Sense::click(),
    );

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let fill = if pressed {
            ui.visuals().selection.bg_fill
        } else {
            visuals.weak_bg_fill
        };
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            fill,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        let pos = rect.center() - ink.center().to_vec2();
        ui.painter().galley(pos, galley, visuals.text_color());
    }

    if let Some(cursor) = ui.visuals().interact_cursor {
        if response.hovered() {
            ui.ctx().set_cursor_icon(cursor);
        }
    }

    response
}

/// Icon-only button.
pub fn icon_button(ui: &mut Ui, icon: &str) -> egui::Response {
    icon_button_widget(ui, icon, false)
}

/// Icon button that stays visually pressed while `pressed` is true.
pub fn icon_toggle(ui: &mut Ui, icon: &str, pressed: bool) -> egui::Response {
    icon_button_widget(ui, icon, pressed)
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
