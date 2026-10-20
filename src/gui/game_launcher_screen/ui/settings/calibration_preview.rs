use eframe::egui::{self, Color32, Stroke};

use super::super::super::display::geometry::PixelRect;
use crate::gui::common::theme;

pub(in crate::gui::game_launcher_screen) fn render(ui: &mut egui::Ui, surface_px: PixelRect) {
    let (circle, _) = super::super::canvas::circle_layout(ui, surface_px);
    let painter = ui.painter();
    painter.rect_filled(ui.max_rect(), 0.0, Color32::BLACK);
    painter.circle_filled(circle.center(), circle.width() / 2.0, Color32::WHITE);
    painter.circle_stroke(
        circle.center(),
        circle.width() / 2.0 - 2.0,
        Stroke::new(4.0_f32, theme::CALIBRATION_GUIDE),
    );
    painter.line_segment(
        [circle.left_center(), circle.right_center()],
        Stroke::new(2.0_f32, theme::CALIBRATION_GUIDE),
    );
}
