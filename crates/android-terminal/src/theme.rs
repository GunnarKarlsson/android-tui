//! Central egui style and theme configuration.

use std::sync::Arc;

use eframe::egui::{
    self, Context, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Ui,
};

const JETBRAINS_MONO: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
const JETBRAINS_MONO_BOLD: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMonoNerdFont-Bold.ttf");

/// All UI colors are defined here once.
pub mod colors {
    use egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(10, 24, 52);
    pub const BG_WIDGET: Color32 = Color32::from_rgb(18, 40, 78);
    pub const BG_HOVER: Color32 = Color32::from_rgb(28, 56, 104);
    pub const BG_EXTREME: Color32 = Color32::from_rgb(6, 16, 36);
    pub const BG_FAINT: Color32 = Color32::from_rgb(16, 36, 72);
    /// Panel card fill.
    pub const PANEL_BG: Color32 = Color32::from_rgb(12, 22, 40);
    /// Thin outline around each panel card.
    pub const PANEL_BORDER: Color32 = Color32::from_rgb(58, 64, 74);

    /// Text and widget foreground.
    pub const OFF_WHITE: Color32 = Color32::from_rgb(232, 232, 228);
    /// Resize handle highlight (gap stays empty when idle).
    pub const PANEL_SPLITTER_HOVER: Color32 = Color32::from_rgb(96, 102, 112);
    /// Header rule inside a panel card.
    pub const PANEL_SEPARATOR: Color32 = Color32::from_rgb(56, 62, 72);
    /// Selected icon toggle / selection fill.
    pub const SELECTION: Color32 = Color32::from_rgb(40, 80, 160);

    pub const ERROR: Color32 = Color32::from_rgb(220, 80, 80);
    pub const LOG_ERROR: Color32 = ERROR;
    pub const LOG_WARNING: Color32 = Color32::from_rgb(220, 180, 60);
    pub const LOG_INFO: Color32 = Color32::from_rgb(120, 180, 255);
    pub const LOG_DEBUG: Color32 = Color32::from_rgb(140, 140, 140);
    pub const LOG_FATAL: Color32 = Color32::from_rgb(255, 60, 60);
    pub const LOG_DEFAULT: Color32 = Color32::GRAY;

    pub const RAM_USED: Color32 = Color32::from_rgb(80, 200, 220);
    pub const RAM_TRACK: Color32 = Color32::from_rgb(40, 50, 65);

    pub const STORAGE_USED: Color32 = Color32::from_rgb(240, 120, 60);
    pub const STORAGE_TRACK: Color32 = Color32::from_rgb(55, 40, 35);
}

/// Corner radius for panel cards.
pub const PANEL_CORNER_RADIUS: u8 = 10;

/// Empty space between adjacent panel cards (canvas shows through).
pub const PANEL_GAP: f32 = 12.0;

/// Padding inside a panel card, around title and body.
pub const PANEL_INNER_PADDING: i8 = 8;

/// Padding between the window edge and panel cards.
pub const PANEL_CANVAS_MARGIN: i8 = 12;

/// Apply app-wide egui styling. Called once at startup from the eframe creation hook.
pub fn configure(ctx: &Context) {
    install_fonts(ctx);

    ctx.all_styles_mut(apply_shared_style);
    ctx.set_visuals_of(egui::Theme::Dark, dark_blue_visuals());
    ctx.set_theme(egui::Theme::Dark);
}

fn apply_shared_style(style: &mut egui::Style) {
    style.spacing.window_margin = egui::Margin::same(PANEL_INNER_PADDING);
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

/// Show an error label using the theme error color.
pub fn error_label(ui: &mut Ui, text: impl AsRef<str>) {
    ui.colored_label(colors::ERROR, text.as_ref());
}

fn panel_separator(ui: &mut Ui) {
    let spacing = ui.spacing().item_spacing.y;
    let (rect, response) = ui.allocate_at_least(egui::vec2(ui.available_width(), spacing), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().hline(
            rect.x_range(),
            rect.center().y,
            egui::Stroke::new(1.0, colors::PANEL_SEPARATOR),
        );
    }
    ui.advance_cursor_after_rect(response.rect);
}

fn dark_blue_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = colors::BG;
    visuals.window_fill = colors::BG;
    visuals.extreme_bg_color = colors::BG_EXTREME;
    visuals.faint_bg_color = colors::BG_FAINT;
    visuals.code_bg_color = colors::BG_EXTREME;
    visuals.override_text_color = Some(colors::OFF_WHITE);
    visuals.selection.bg_fill = colors::SELECTION;

    visuals.widgets.noninteractive.bg_fill = colors::BG;
    visuals.widgets.noninteractive.weak_bg_fill = colors::BG;
    visuals.widgets.noninteractive.fg_stroke.color = colors::OFF_WHITE;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

    visuals.widgets.inactive.bg_fill = colors::BG_WIDGET;
    visuals.widgets.inactive.weak_bg_fill = colors::BG_WIDGET;
    visuals.widgets.inactive.fg_stroke.color = colors::OFF_WHITE;

    visuals.widgets.hovered.bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.fg_stroke.color = colors::OFF_WHITE;

    visuals.widgets.active.bg_fill = colors::BG_HOVER;
    visuals.widgets.active.weak_bg_fill = colors::BG_HOVER;
    visuals.widgets.active.fg_stroke.color = colors::OFF_WHITE;

    visuals.widgets.open.bg_fill = colors::BG_WIDGET;
    visuals.widgets.open.weak_bg_fill = colors::BG_WIDGET;
    visuals.widgets.open.fg_stroke.color = colors::OFF_WHITE;

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

/// Dark canvas behind panel cards.
pub fn shell_frame(_ctx: &Context) -> egui::Frame {
    egui::Frame::NONE.fill(colors::BG_EXTREME)
}

/// Insets panel cards from the window edge.
pub fn canvas_margin_frame() -> egui::Frame {
    egui::Frame::NONE.inner_margin(egui::Margin::same(PANEL_CANVAS_MARGIN))
}

fn panel_frame(ui: &Ui) -> egui::Frame {
    egui::Frame::default()
        .fill(colors::PANEL_BG)
        .stroke(egui::Stroke::new(1.0, colors::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(PANEL_CORNER_RADIUS))
        .inner_margin(panel_padding(ui))
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
    panel_frame(ui)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(title);
                add_header_actions(ui);
            });
            panel_separator(ui);
            add_body(ui)
        })
        .inner
}
