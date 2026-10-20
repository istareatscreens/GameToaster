use eframe::egui::{self, Color32, CornerRadius, Margin};

use super::palette::{BORDER, CANVAS, ICON_TILE, LIST_PANEL, ROW_CREAM, TOPBAR_CORAL};

fn soft_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 2],
        blur: 9,
        spread: 0,
        color: Color32::from_black_alpha(28),
    }
}

pub(in crate::gui::common) fn modal() -> egui::Frame {
    egui::Frame::new()
        .fill(ROW_CREAM)
        .stroke(BORDER)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(20))
        .shadow(egui::epaint::Shadow {
            offset: [0, 6],
            blur: 22,
            spread: 1,
            color: Color32::from_black_alpha(70),
        })
}

pub(in crate::gui::common) fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(ICON_TILE)
        .stroke(BORDER)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(8))
}

pub(in crate::gui::common) fn compact_card() -> egui::Frame {
    card().inner_margin(Margin::symmetric(8, 4))
}

pub(in crate::gui::common) fn section() -> egui::Frame {
    egui::Frame::new()
        .fill(ROW_CREAM)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 8))
}

pub fn title_bar() -> egui::Frame {
    egui::Frame::new().fill(TOPBAR_CORAL)
}

pub fn canvas(margin: Margin) -> egui::Frame {
    egui::Frame::new().fill(CANVAS).inner_margin(margin)
}

pub fn list_panel() -> egui::Frame {
    egui::Frame::new()
        .fill(LIST_PANEL)
        .stroke(BORDER)
        .corner_radius(CornerRadius::same(12))
        .shadow(soft_shadow())
        .inner_margin(Margin::same(8))
}

pub fn list_row() -> egui::Frame {
    egui::Frame::new()
        .fill(ROW_CREAM)
        .stroke(BORDER)
        .corner_radius(CornerRadius::same(10))
        .shadow(soft_shadow())
        .inner_margin(Margin::symmetric(14, 8))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/theme/frames__tests.rs"]
mod tests;
