use super::*;
use crate::gui::common::theme;

type Show =
    fn(&mut egui::Ui, Box<dyn FnOnce(&mut egui::Ui) -> egui::Rect>) -> InnerResponse<egui::Rect>;

fn drawn(show: Show, width: f32) -> (egui::epaint::RectShape, egui::Rect, egui::Rect) {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut card_rect = None;
    let mut label_rect = None;
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        ui.set_width(width);
        let response = show(ui, Box::new(|ui| ui.label("content").rect));
        card_rect = Some(response.response.rect);
        label_rect = Some(response.inner);
    });
    let card_rect = card_rect.unwrap();
    let frame = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.rect == card_rect => Some(shape.clone()),
            _ => None,
        })
        .expect("the card paints its frame over its whole rect");
    (frame, card_rect, label_rect.unwrap())
}

fn standard(
    ui: &mut egui::Ui,
    body: Box<dyn FnOnce(&mut egui::Ui) -> egui::Rect>,
) -> InnerResponse<egui::Rect> {
    card(ui, body)
}

fn compact(
    ui: &mut egui::Ui,
    body: Box<dyn FnOnce(&mut egui::Ui) -> egui::Rect>,
) -> InnerResponse<egui::Rect> {
    compact_card(ui, body)
}

fn grouped(
    ui: &mut egui::Ui,
    body: Box<dyn FnOnce(&mut egui::Ui) -> egui::Rect>,
) -> InnerResponse<egui::Rect> {
    section(ui, body)
}

#[test]
fn each_card_paints_its_look_and_insets_its_content_by_its_margin() {
    let cards: [(Show, egui::Frame); 3] = [
        (standard, frames::card()),
        (compact, frames::compact_card()),
        (grouped, frames::section()),
    ];
    for (show, look) in cards {
        let (frame, card_rect, label_rect) = drawn(show, 300.0);
        assert_eq!(
            (frame.fill, frame.stroke, frame.corner_radius),
            (look.fill, look.stroke, look.corner_radius)
        );
        let margin = look.total_margin();
        assert!((label_rect.left() - card_rect.left() - margin.left).abs() < 0.01);
        assert!((label_rect.top() - card_rect.top() - margin.top).abs() < 0.01);
    }
}

#[test]
fn a_section_spans_the_width_it_is_given() {
    let (_, section_rect, _) = drawn(grouped, 300.0);
    assert!(
        (section_rect.width() - 300.0).abs() < 0.01,
        "{section_rect:?}"
    );
    let (_, card_rect, _) = drawn(standard, 300.0);
    assert!(
        card_rect.width() < 300.0,
        "a card wraps its content: {card_rect:?}"
    );
}

#[test]
fn a_card_leaves_the_next_widget_style_unchanged() {
    let context = egui::Context::default();
    theme::apply(&context);
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        let before = (**ui.style()).clone();
        card(ui, |ui| ui.label("content"));
        compact_card(ui, |ui| ui.label("content"));
        section(ui, |ui| ui.label("content"));
        assert_eq!(**ui.style(), before);
    });
}
