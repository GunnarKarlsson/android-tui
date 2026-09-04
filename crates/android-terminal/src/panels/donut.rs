use eframe::egui;

use crate::theme;

pub(crate) fn show_usage_donut(
    ui: &mut egui::Ui,
    scroll_id: egui::Id,
    fraction: f32,
    used_label: String,
    total_label: String,
    track_color: egui::Color32,
    used_color: egui::Color32,
) {
    let percent = (fraction * 100.0).round() as u32;

    egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.set_width(ui.available_width());

                let diameter = ui.available_width().clamp(80.0, 160.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::hover());

                let painter = ui.painter_at(rect);
                let center = rect.center();
                let radius = diameter * 0.38;
                let stroke_width = diameter * 0.12;

                paint_usage_donut(
                    &painter,
                    center,
                    radius,
                    stroke_width,
                    fraction,
                    track_color,
                    used_color,
                );
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    format!("{percent}%"),
                    egui::FontId::proportional(28.0),
                    theme::colors::OFF_WHITE,
                );

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("{used_label} / {total_label}"))
                        .color(theme::colors::LOG_DEBUG)
                        .size(12.0),
                );
            });
        });
}

fn paint_usage_donut(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    stroke_width: f32,
    fraction: f32,
    track_color: egui::Color32,
    used_color: egui::Color32,
) {
    use std::f32::consts::TAU;

    let start = -TAU / 4.0;
    let used = fraction.clamp(0.0, 1.0);

    paint_ring_arc(
        painter,
        center,
        radius,
        start,
        TAU,
        stroke_width,
        track_color,
    );
    if used > 0.0 {
        paint_ring_arc(
            painter,
            center,
            radius,
            start,
            used * TAU,
            stroke_width,
            used_color,
        );
    }
}

fn paint_ring_arc(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    start: f32,
    sweep: f32,
    stroke_width: f32,
    color: egui::Color32,
) {
    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: arc_points(center, radius, start, sweep, 64),
        closed: false,
        fill: egui::Color32::TRANSPARENT,
        stroke: egui::epaint::PathStroke::new(stroke_width, color),
    }));
}

fn arc_points(
    center: egui::Pos2,
    radius: f32,
    start: f32,
    sweep: f32,
    steps: usize,
) -> Vec<egui::Pos2> {
    (0..=steps)
        .map(|step| {
            let angle = start + sweep * step as f32 / steps as f32;
            center + egui::vec2(angle.cos(), angle.sin()) * radius
        })
        .collect()
}
