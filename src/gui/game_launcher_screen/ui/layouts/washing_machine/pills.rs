use eframe::egui::{self, Color32, FontId};

use super::metrics::{CREDIT_READOUT_BOTTOM_OFFSET, LOADING_STATUS_TOP_OFFSET};
use crate::gui::common::theme;

pub(super) fn paint_loading_status(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    scale: f32,
    status: &str,
) {
    paint_pill(
        painter,
        center,
        radius,
        scale,
        status,
        -LOADING_STATUS_TOP_OFFSET,
        theme::PATH_BLUE,
    );
}

pub(super) fn paint_credit_readout(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    scale: f32,
    credits: &str,
) {
    paint_pill(
        painter,
        center,
        radius,
        scale,
        credits,
        CREDIT_READOUT_BOTTOM_OFFSET,
        theme::PRIMARY_ORANGE,
    );
}

fn paint_pill(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    scale: f32,
    text: &str,
    offset_units: f32,
    color: Color32,
) {
    if text.is_empty() {
        return;
    }
    let pill_y = center.y + offset_units * scale;

    let dy = offset_units.abs() * scale;
    let chord_half = (radius * radius - dy * dy).max(0.0).sqrt();
    let pad = egui::vec2(16.0 * scale, 8.0 * scale);
    let max_box_half = (chord_half - 24.0 * scale).max(20.0 * scale);
    let max_text_width = (2.0 * max_box_half - 2.0 * pad.x).max(10.0);

    let mut job = egui::text::LayoutJob::single_section(
        text.to_string(),
        egui::text::TextFormat {
            font_id: FontId::proportional((28.0 * scale).max(14.0)),
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: max_text_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = painter.layout_job(job);

    let box_rect =
        egui::Rect::from_center_size(egui::pos2(center.x, pill_y), galley.size() + 2.0 * pad);
    painter.rect(
        box_rect,
        egui::CornerRadius::same((12.0 * scale).clamp(4.0, 20.0) as u8),
        Color32::WHITE,
        theme::BORDER,
        egui::StrokeKind::Inside,
    );
    painter.galley(box_rect.center() - galley.size() / 2.0, galley, color);
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/ui/layouts/washing_machine/pills__tests.rs"]
mod tests;
