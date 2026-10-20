use super::*;
use crate::gui::common::harness::{locale, painted_shapes};
use crate::gui::common::theme;

#[test]
fn the_field_leaves_the_browse_button_exactly_the_room_it_takes() {
    let _locale = locale("en");

    let body = egui::Rect::from_min_size(egui::pos2(30.0, 30.0), egui::vec2(490.0, 100.0));
    let context = egui::Context::default();
    theme::apply(&context);
    let mut watch_exe = String::new();
    let output = context.run_ui(egui::RawInput::default(), |root| {
        root.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            theme::apply_widget_visuals(ui);
            show(ui, &mut watch_exe, "game.exe");
        });
    });
    let filled = |fill: egui::Color32| {
        painted_shapes(&output)
            .into_iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect) if rect.fill == fill => Some(rect.rect),
                _ => None,
            })
            .unwrap_or_else(|| panic!("nothing is filled with {fill:?}"))
    };
    let (field, browse) = (filled(theme::ICON_TILE), filled(theme::PRIMARY_ORANGE));
    let spacing = context.global_style().spacing.item_spacing.x;
    assert!((field.left() - body.left()).abs() < 0.5, "{field:?}");
    assert!(
        (browse.left() - field.right() - spacing).abs() < 0.5,
        "{field:?} {browse:?}"
    );
    assert!(
        (browse.right() - body.right()).abs() < 0.5,
        "the Browse button ends at the body's edge: {browse:?} {body:?}"
    );
}
