use super::*;
use crate::gui::common::theme::{BORDER, CANVAS, ICON_TILE, LIST_PANEL, ROW_CREAM, TOPBAR_CORAL};
use eframe::egui::{epaint::Shadow, Stroke};

fn look(frame: egui::Frame) -> (Color32, Stroke, CornerRadius, Margin, Shadow) {
    (
        frame.fill,
        frame.stroke,
        frame.corner_radius,
        frame.inner_margin,
        frame.shadow,
    )
}

fn soft() -> Shadow {
    Shadow {
        offset: [0, 2],
        blur: 9,
        spread: 0,
        color: Color32::from_black_alpha(28),
    }
}

#[test]
fn the_dialog_and_card_looks_are_exact() {
    assert_eq!(
        look(modal()),
        (
            ROW_CREAM,
            BORDER,
            CornerRadius::same(12),
            Margin::same(20),
            Shadow {
                offset: [0, 6],
                blur: 22,
                spread: 1,
                color: Color32::from_black_alpha(70),
            },
        )
    );
    assert_eq!(
        look(card()),
        (
            ICON_TILE,
            BORDER,
            CornerRadius::same(6),
            Margin::same(8),
            Shadow::NONE
        )
    );
    assert_eq!(
        look(compact_card()),
        (
            ICON_TILE,
            BORDER,
            CornerRadius::same(6),
            Margin::symmetric(8, 4),
            Shadow::NONE
        )
    );
    assert_eq!(
        look(section()),
        (
            ROW_CREAM,
            Stroke::NONE,
            CornerRadius::same(8),
            Margin::symmetric(12, 8),
            Shadow::NONE
        )
    );
}

#[test]
fn the_main_window_looks_are_exact() {
    assert_eq!(
        look(title_bar()),
        (
            TOPBAR_CORAL,
            Stroke::NONE,
            CornerRadius::ZERO,
            Margin::ZERO,
            Shadow::NONE
        )
    );
    let margin = Margin {
        left: 22,
        right: 22,
        top: 16,
        bottom: 16,
    };
    assert_eq!(
        look(canvas(margin)),
        (
            CANVAS,
            Stroke::NONE,
            CornerRadius::ZERO,
            margin,
            Shadow::NONE
        )
    );
    assert_eq!(
        look(list_panel()),
        (
            LIST_PANEL,
            BORDER,
            CornerRadius::same(12),
            Margin::same(8),
            soft()
        )
    );
    assert_eq!(
        look(list_row()),
        (
            ROW_CREAM,
            BORDER,
            CornerRadius::same(10),
            Margin::symmetric(14, 8),
            soft()
        )
    );
}
