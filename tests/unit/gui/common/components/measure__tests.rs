use super::*;
use crate::gui::common::components::{action, checkbox};
use crate::gui::common::harness::{click, painted_shapes};
use crate::gui::common::theme;

#[test]
fn a_measured_widget_has_the_size_it_takes_when_added() {
    let context = egui::Context::default();
    theme::apply(&context);
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        ui.horizontal(|ui| {
            let measured = measure(ui, |ui| action(ui, "Browse"));
            assert_eq!(measured, action(ui, "Browse").rect.size());
            let measured = measure(ui, |ui| checkbox(ui, &mut false, "Setting"));
            assert_eq!(measured, checkbox(ui, &mut false, "Setting").rect.size());
        });
    });
}

fn browse_row(
    context: &egui::Context,
    input: egui::RawInput,
    measured: bool,
) -> (egui::FullOutput, bool, bool, egui::Rect) {
    let (mut unseen_clicked, mut clicked, mut rect) = (false, false, egui::Rect::NOTHING);
    let output = context.run_ui(input, |ui| {
        ui.horizontal(|ui| {
            if measured {
                let _ = measure(ui, |ui| {
                    let unseen = action(ui, "Browse");
                    unseen_clicked = unseen.clicked();
                    unseen
                });
            }
            let shown = action(ui, "Browse");
            clicked = shown.clicked();
            rect = shown.rect;
        });
    });
    (output, unseen_clicked, clicked, rect)
}

fn visible(output: &egui::FullOutput) -> Vec<egui::Shape> {
    painted_shapes(output)
        .into_iter()
        .filter(|shape| !matches!(shape, egui::Shape::Noop))
        .collect()
}

#[test]
fn measuring_paints_nothing_and_takes_no_room_and_no_input() {
    let context = egui::Context::default();
    theme::apply(&context);
    let (plain, _, _, plain_rect) = browse_row(&context, egui::RawInput::default(), false);

    let context = egui::Context::default();
    theme::apply(&context);
    let (output, _, _, rect) = browse_row(&context, egui::RawInput::default(), true);
    assert_eq!(rect, plain_rect, "the measurement takes no room");
    assert!(
        visible(&output) == visible(&plain),
        "the measurement paints nothing"
    );

    let (_, unseen_clicked, clicked, _) =
        browse_row(&context, click(rect.center(), None, 0.1), true);
    assert!(!unseen_clicked, "the measured copy takes no input");
    assert!(clicked, "the shown action still takes its click");
}
