use eframe::egui::{self, Vec2};

use super::super::display::geometry::PixelRect;
use super::super::surface::circle::metrics::CIRCLE_UNITS;

pub(super) fn circle_layout(ui: &egui::Ui, surface_px: PixelRect) -> (egui::Rect, f32) {
    let ppp = ui.ctx().pixels_per_point().max(0.5);
    let circle = egui::Rect::from_min_size(
        egui::pos2(surface_px.x as f32 / ppp, surface_px.y as f32 / ppp),
        Vec2::splat(surface_px.width as f32 / ppp),
    );
    let scale = circle.width() / CIRCLE_UNITS;
    (circle, scale)
}
