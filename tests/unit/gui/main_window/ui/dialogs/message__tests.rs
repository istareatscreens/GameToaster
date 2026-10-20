use super::super::harness;
use super::super::Dialog;
use super::*;
use crate::gui::common::harness::painted_shapes;
use crate::gui::common::theme;

fn colour_of(dialog: Dialog, message: &str) -> egui::Color32 {
    let shown = harness::show(dialog);
    painted_shapes(&shown.output)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.text() == message => Some(text.fallback_color),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{message:?} is not painted"))
}

#[test]
fn error_offers_ok_then_copy_and_copy_takes_the_title_and_the_message() {
    let mut shown = harness::show(Dialog::error("Launch failed", "The handle is invalid"));
    assert_eq!(shown.actions().len(), 2);
    assert!(matches!(shown.click_action(1), DialogEvent::None));
    let copied = shown.output.platform_output.commands.iter().any(|command| {
        matches!(command, egui::OutputCommand::CopyText(text)
            if text == "Launch failed\nThe handle is invalid")
    });
    assert!(copied, "{:?}", shown.output.platform_output.commands);
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
}

#[test]
fn a_notice_offers_the_same_ok_and_copy() {
    let mut shown = harness::show(Dialog::notice("Settings converted", "Kept at C:\\State"));
    assert_eq!(shown.actions().len(), 2);
    assert!(matches!(shown.click_action(1), DialogEvent::None));
    let copied = shown.output.platform_output.commands.iter().any(|command| {
        matches!(command, egui::OutputCommand::CopyText(text)
            if text == "Settings converted\nKept at C:\\State")
    });
    assert!(copied, "{:?}", shown.output.platform_output.commands);
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
}

#[test]
fn an_error_reads_in_the_error_colour_and_a_notice_does_not() {
    assert_eq!(
        colour_of(
            Dialog::error("Title", "Something failed"),
            "Something failed"
        ),
        theme::ERROR
    );
    assert_ne!(
        colour_of(
            Dialog::notice("Title", "Something changed"),
            "Something changed"
        ),
        theme::ERROR
    );
}
