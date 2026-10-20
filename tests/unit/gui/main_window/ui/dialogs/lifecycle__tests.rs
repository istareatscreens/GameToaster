use super::super::harness::{self, game_entry};
use super::super::Dialog;
use super::*;
use crate::gui::common::harness::{assert_pinned, locale};

#[test]
fn remove_offers_cancel_then_remove() {
    let mut shown = harness::show(Dialog::Remove(game_entry()));
    assert_eq!(shown.actions().len(), 2);
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
    match shown.click_action(1) {
        DialogEvent::RemoveGame(game) => assert_eq!(game.id, game_entry().id),
        other => panic!("{other:?}"),
    }
}

#[test]
fn edit_label_saves_only_a_valid_submission() {
    let mut draft = PresentationDraft::new(game_entry(), None, None);
    draft.label = "  Renamed  ".into();
    let mut shown = harness::show(Dialog::Presentation(draft));
    assert_eq!(shown.actions().len(), 2);
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
    match shown.click_action(1) {
        DialogEvent::SavePresentation {
            id,
            label,
            source,
            clear_image,
        } => {
            assert_eq!(id, game_entry().id);
            assert_eq!(label, "Renamed");
            assert_eq!((source, clear_image), (None, false));
        }
        other => panic!("{other:?}"),
    }

    let mut blank = PresentationDraft::new(game_entry(), None, None);
    blank.label = "   ".into();
    let mut shown = harness::show(Dialog::Presentation(blank));
    assert!(
        matches!(shown.click_action(1), DialogEvent::None),
        "Save is disabled without a label"
    );
}

#[test]
fn edit_label_pins_its_error_under_the_title() {
    let _locale = locale("en");
    let draft = PresentationDraft::new(game_entry(), None, Some("artwork unreadable".into()));
    let shown = harness::show(Dialog::Presentation(draft));
    assert_pinned(
        &shown.output,
        shown.window(),
        &t!("title.edit_label"),
        "artwork unreadable",
    );
}

#[test]
fn open_config_offers_cancel() {
    let mut shown = harness::show(Dialog::ConfigMenu(game_entry()));
    assert_eq!(shown.actions().len(), 1);
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
}

fn label(shown: &harness::Shown) -> &str {
    match &shown.dialog {
        Dialog::Presentation(draft) => &draft.label,
        _ => unreachable!("the dialog is still Edit label"),
    }
}

#[test]
fn edit_label_focuses_its_label_field_when_it_opens() {
    let mut shown = harness::show(Dialog::Presentation(PresentationDraft::new(
        game_entry(),
        None,
        None,
    )));
    let _ = shown.type_text("X");
    assert_eq!(
        label(&shown),
        "OriginalX",
        "typing goes into the label field"
    );
}

#[test]
fn tab_moves_focus_off_the_label_field_for_good() {
    let mut shown = harness::show(Dialog::Presentation(PresentationDraft::new(
        game_entry(),
        None,
        None,
    )));
    let _ = shown.press(egui::Key::Tab);
    for _ in 0..3 {
        let _ = shown.idle();
    }
    assert!(shown.focused().is_some(), "another widget has the focus");
    let _ = shown.type_text("X");
    assert_eq!(
        label(&shown),
        "Original",
        "the label field no longer takes typing"
    );
}

#[test]
fn enter_with_a_non_blank_label_saves_the_presentation() {
    let mut draft = PresentationDraft::new(game_entry(), None, None);
    draft.label = "  Renamed  ".into();
    let mut shown = harness::show(Dialog::Presentation(draft));
    match shown.press(egui::Key::Enter) {
        DialogEvent::SavePresentation { label, .. } => assert_eq!(label, "Renamed"),
        other => panic!("{other:?}"),
    }
}
