use super::*;
use crate::gui::common::harness::{
    click, filled_rects, painted_elisions, painted_on_both_surfaces, painted_texts,
};
use crate::gui::common::theme;

const CHOICES: [(u8, &str); 3] = [(1, "One"), (2, "Two"), (3, "Three")];

fn combo(current: &mut u8) -> Combo<'_, u8> {
    Combo::new("combo-under-test", current)
        .choices(CHOICES)
        .placeholder("Pick one")
}

fn closed_text(current: u8) -> Vec<String> {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut value = current;
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        combo(&mut value).show(ui);
    });
    painted_texts(&output)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

fn resending(current: &mut u8) -> Combo<'_, u8> {
    combo(current).resend_current()
}

fn pick(build: fn(&mut u8) -> Combo<'_, u8>, initial: u8, label: &str) -> (u8, bool) {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut value = initial;
    let mut button = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        button = Some(build(&mut value).show(ui));
    });
    let button = button.unwrap();
    egui::Popup::open_id(&context, button.id.with("popup"));

    let mut output = None;
    for frame in 1..=3 {
        let input = egui::RawInput {
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        output = Some(context.run_ui(input, |ui| {
            build(&mut value).show(ui);
        }));
    }
    let target = painted_texts(&output.unwrap())
        .into_iter()
        .find(|(text, rect)| text == label && !button.rect.contains(rect.center()))
        .map(|(_, rect)| rect.center())
        .expect("the open popup paints the choice");
    let click = click(target, None, 0.4);
    let mut changed = false;
    let _ = context.run_ui(click, |ui| {
        changed = build(&mut value).show(ui).changed();
    });
    (value, changed)
}

#[test]
fn the_closed_box_shows_the_current_choice_or_else_the_placeholder() {
    assert_eq!(closed_text(2), ["Two"]);
    assert_eq!(closed_text(9), ["Pick one"]);
}

#[test]
fn picking_another_choice_sets_the_value_and_reports_a_change() {
    assert_eq!(pick(combo, 1, "Three"), (3, true));
}

#[test]
fn picking_the_current_choice_reports_no_change() {
    assert_eq!(pick(combo, 2, "Two"), (2, false));
}

#[test]
fn resend_current_reports_a_pick_of_the_current_choice_too() {
    assert_eq!(pick(resending, 2, "Two"), (2, true));
    assert_eq!(pick(resending, 1, "Three"), (3, true));
}

#[test]
fn a_fixed_width_box_is_that_wide_and_elides_a_longer_text_on_one_line() {
    const LONG: &str = "A choice whose label is far wider than its box";
    let context = egui::Context::default();
    theme::apply(&context);
    let mut boxes = Vec::new();
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        for current in [1u8, 2] {
            let mut value = current;
            let shown = Combo::new(("fixed", current), &mut value)
                .choices([(1u8, "One"), (2, LONG)])
                .width(180.0)
                .show(ui);
            boxes.push(shown.rect);
        }
    });
    for rect in &boxes {
        assert!((rect.width() - 180.0).abs() < 0.5, "{rect:?}");
    }
    assert_eq!(
        boxes[0].height(),
        boxes[1].height(),
        "the longer text stays on one line"
    );
    let texts = painted_elisions(&output);
    assert!(texts.contains(&("One".to_owned(), false)), "{texts:?}");
    assert!(texts.contains(&(LONG.to_owned(), true)), "{texts:?}");
}

#[test]
fn the_closed_box_looks_the_same_on_any_surface() {
    let [main_window, dialog] = painted_on_both_surfaces(|ui| {
        combo(&mut 2).show(ui);
    });
    assert!(main_window == dialog, "{main_window:#?}\n{dialog:#?}");

    assert_eq!(
        filled_rects(&dialog),
        [(theme::ICON_TILE, theme::BORDER, egui::CornerRadius::same(6))]
    );
}

#[test]
fn full_width_takes_the_available_width() {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut value = 1;
    let mut widths = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        ui.allocate_ui(egui::vec2(300.0, 200.0), |ui| {
            let full = combo(&mut value).full_width().show(ui).rect.width();
            let natural = Combo::new("natural", &mut value)
                .choices(CHOICES)
                .show(ui)
                .rect
                .width();
            widths = Some((full, natural));
        });
    });
    let (full, natural) = widths.unwrap();
    assert!((full - 300.0).abs() < 0.5, "{full}");
    assert!(natural < 300.0, "{natural}");
}

fn first_id(mut show: impl FnMut(&mut egui::Ui) -> egui::Id) -> egui::Id {
    let context = egui::Context::default();
    let mut id = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| id = Some(show(ui)));
    id.unwrap()
}

#[test]
fn an_unlabelled_box_keeps_eguis_id() {
    assert_eq!(
        first_id(|ui| Combo::new("salt", &mut 1).choices(CHOICES).show(ui).id),
        first_id(|ui| {
            egui::ComboBox::from_id_salt("salt")
                .show_ui(ui, |_| {})
                .response
                .id
        }),
    );
}

fn labelled(current: &mut u8) -> Combo<'_, u8> {
    Combo::labelled("Monitor", current)
        .choices(CHOICES)
        .placeholder("Pick one")
}

#[test]
fn a_labelled_box_keeps_its_id_from_frame_to_frame_so_its_list_works() {
    let context = egui::Context::default();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let _ = context.run_ui(egui::RawInput::default(), |ui| {
            ids.push(labelled(&mut 1).show(ui).id);
            ids.push(
                Combo::labelled("Profile", &mut 1)
                    .choices(CHOICES)
                    .show(ui)
                    .id,
            );
        });
    }
    assert_eq!((ids[0], ids[1]), (ids[2], ids[3]));
    assert_ne!(ids[0], ids[1]);
    assert_eq!(pick(labelled, 1, "Three"), (3, true));
}

#[test]
fn a_box_and_its_label_stay_within_the_row_however_long_the_value() {
    const LONG: &str = "A value far longer than the room its box has in this row";
    let row = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(320.0, 300.0));
    let context = egui::Context::default();
    theme::apply(&context);
    let mut boxes = Vec::new();
    let output = context.run_ui(egui::RawInput::default(), |root| {
        root.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
            let (mut first, mut second) = (1u8, 1u8);
            for combo in [
                Combo::new("unlabelled", &mut first),
                Combo::labelled("Monitor", &mut second),
            ] {
                boxes.push(combo.choices([(1u8, LONG)]).show(ui).rect);
            }
        });
    });

    for rect in &boxes {
        assert!(rect.right() <= row.right() + 0.5, "{rect:?} past {row:?}");
    }
    let texts = painted_elisions(&output);
    assert_eq!(
        texts
            .iter()
            .filter(|text| **text == (LONG.to_owned(), true))
            .count(),
        2,
        "{texts:?}"
    );
    assert!(texts.contains(&("Monitor".to_owned(), false)), "{texts:?}");
}

#[test]
fn a_label_wider_than_its_row_leaves_the_box_its_least_width() {
    let row = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(120.0, 100.0));
    let context = egui::Context::default();
    theme::apply(&context);
    let mut shown = None;
    let _ = context.run_ui(egui::RawInput::default(), |root| {
        root.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
            shown = Some(
                Combo::labelled("A label far wider than the row it is in", &mut 1u8)
                    .choices(CHOICES)
                    .show(ui),
            );
        });
    });
    assert!(shown.unwrap().rect.width() > 0.0);
}

#[test]
fn a_box_that_turns_disabled_closes_its_open_list_and_takes_no_pick() {
    let show = |ui: &mut egui::Ui, value: &mut u8, enabled: bool| {
        ui.add_enabled_ui(enabled, |ui| combo(value).show(ui)).inner
    };
    let context = egui::Context::default();
    theme::apply(&context);
    let mut value = 1u8;
    let mut button = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        button = Some(show(ui, &mut value, true));
    });
    let button = button.unwrap();
    egui::Popup::open_id(&context, button.id.with("popup"));
    let frame = |time: f64| egui::RawInput {
        time: Some(time),
        ..Default::default()
    };
    let mut output = None;
    for time in [0.1, 0.2, 0.3] {
        output = Some(context.run_ui(frame(time), |ui| {
            show(ui, &mut value, true);
        }));
    }
    let three = painted_texts(&output.unwrap())
        .into_iter()
        .find(|(text, rect)| text == "Three" && !button.rect.contains(rect.center()))
        .map(|(_, rect)| rect.center())
        .expect("the open list paints its choices");
    let mut output = None;
    for time in [0.4, 0.5] {
        output = Some(context.run_ui(frame(time), |ui| {
            show(ui, &mut value, false);
        }));
    }
    assert!(
        painted_texts(&output.unwrap())
            .iter()
            .all(|(text, _)| text != "Three"),
        "the list closes"
    );
    let mut changed = false;
    let _ = context.run_ui(click(three, None, 0.8), |ui| {
        changed = show(ui, &mut value, false).changed();
    });
    assert_eq!((value, changed), (1, false));
}
