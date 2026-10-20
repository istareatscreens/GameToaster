use super::*;
use crate::gui::common::harness::{painted_elisions, painted_texts};
use crate::gui::common::theme;

const ROOM: egui::Rect = egui::Rect::from_min_max(egui::pos2(20.0, 20.0), egui::pos2(420.0, 120.0));

fn drawn<R>(row: impl FnOnce(&mut egui::Ui) -> R) -> (egui::FullOutput, R) {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut result = None;
    let mut row = Some(row);
    let output = context.run_ui(egui::RawInput::default(), |root| {
        if let Some(row) = row.take() {
            root.scope_builder(egui::UiBuilder::new().max_rect(ROOM), |ui| {
                result = Some(row(ui));
            });
        }
    });
    (output, result.expect("the row was drawn"))
}

#[test]
fn a_field_fills_the_row_and_the_action_ends_at_its_edge() {
    let mut value = String::new();
    let spacing = 10.0;
    let (_, (field, browse)) = drawn(|ui| {
        assert_eq!(ui.spacing().item_spacing.x, spacing);
        action_row(ui, "Browse", |ui| {
            ui.add(egui::TextEdit::singleline(&mut value).desired_width(f32::INFINITY))
        })
    });
    assert!((field.rect.left() - ROOM.left()).abs() < 0.5, "{field:?}");
    assert!(
        (browse.rect.left() - field.rect.right() - spacing).abs() < 0.5,
        "{field:?} {browse:?}"
    );
    assert!(
        (browse.rect.right() - ROOM.right()).abs() < 0.5,
        "{browse:?}"
    );
}

#[test]
fn a_short_label_keeps_the_action_right_after_it() {
    let (_, (label, action)) = drawn(|ui| {
        action_row(ui, "Choose", |ui| {
            ui.add(egui::Label::new("Short").truncate())
        })
    });
    assert!(
        (action.rect.left() - label.rect.right() - 10.0).abs() < 0.5,
        "{label:?} {action:?}"
    );
    assert!(action.rect.right() < ROOM.right() - 100.0, "{action:?}");
}

#[test]
fn a_long_label_is_elided_and_the_row_stays_within_its_ui() {
    const LONG: &str = "C:/A/folder/name/so/long/that/the/label/cannot/fit/beside/its/action.bat";
    let (output, (_, action)) =
        drawn(|ui| action_row(ui, "Choose", |ui| ui.add(egui::Label::new(LONG).truncate())));
    assert!(
        painted_elisions(&output).contains(&(LONG.to_owned(), true)),
        "{:?}",
        painted_texts(&output)
    );

    assert!(action.rect.right() <= ROOM.right() + 0.5, "{action:?}");
}
