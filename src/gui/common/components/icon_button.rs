use eframe::egui::{self, Color32, Response, Sense, Vec2};
use egui_phosphor::regular as icons;

use crate::gui::common::theme;

pub const TILE_SIZE: Vec2 = Vec2::new(46.0, 40.0);

const TILE_RADIUS: f32 = 7.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TileTone {
    Plain,

    Danger,
}

pub fn tile_button(
    ui: &mut egui::Ui,
    glyph: &str,
    fill: Color32,
    tone: TileTone,
    enabled: bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        TILE_SIZE,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let background = if !enabled {
        fill.gamma_multiply(0.5)
    } else if response.hovered() {
        match tone {
            TileTone::Danger => theme::CLOSE_HOVER,
            TileTone::Plain => fill.gamma_multiply(1.1),
        }
    } else {
        fill
    };
    ui.painter().rect_filled(rect, TILE_RADIUS, background);
    ui.painter()
        .rect_stroke(rect, TILE_RADIUS, theme::BORDER, egui::StrokeKind::Inside);
    let ink = if !enabled {
        theme::MUTED
    } else {
        match tone {
            TileTone::Danger => theme::ERROR,
            TileTone::Plain => theme::INK,
        }
    };
    theme::paint_glyph(
        ui.painter(),
        rect.center(),
        glyph,
        theme::font_size::ICON,
        ink,
    );
    response
}

pub fn window_close_button(ui: &mut egui::Ui, size: Vec2) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::CLOSE_HOVER);
    }
    theme::paint_glyph(
        ui.painter(),
        rect.center(),
        icons::X,
        theme::font_size::LARGE,
        theme::INK,
    );
    response
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/icon_button__tests.rs"]
mod tests;
