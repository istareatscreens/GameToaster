use super::*;
use crate::gui::common::harness::{
    click, filled_rects, painted_on_both_surfaces, painted_shapes, second_frame,
};

#[test]
fn shared_checkbox_and_radio_are_interactive_widgets() {
    let context = egui::Context::default();
    let mut checked = false;
    let mut choice = 1u8;
    let mut responses = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        responses = Some((
            checkbox(ui, &mut checked, "Checkbox"),
            radio(ui, &mut choice, 2u8, "Radio"),
        ));
    });
    let (checkbox_response, radio_response) = responses.unwrap();
    assert!(checkbox_response.sense.senses_click());
    assert!(radio_response.sense.senses_click());
    assert!(!checked);
    assert_eq!(choice, 1);
}

#[test]
fn the_standard_checkbox_and_radio_look_the_same_on_any_surface() {
    let [main_window, dialog] = painted_on_both_surfaces(|ui| {
        checkbox(ui, &mut true, "Checkbox");
        radio(ui, &mut 1u8, 1u8, "Radio");
    });
    assert!(main_window == dialog, "{main_window:#?}\n{dialog:#?}");

    assert_eq!(
        filled_rects(&dialog),
        [(theme::ICON_TILE, theme::BORDER, egui::CornerRadius::same(6))]
    );
    let circles: Vec<_> = dialog
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Circle(circle) => Some((circle.fill, circle.stroke)),
            _ => None,
        })
        .collect();
    assert_eq!(
        circles,
        [
            (theme::ICON_TILE, theme::BORDER),
            (theme::INK, egui::Stroke::NONE)
        ]
    );
}

fn pill(
    output: &egui::FullOutput,
    rect: egui::Rect,
) -> (
    Vec<(Color32, egui::CornerRadius)>,
    bool,
    Vec<egui::epaint::CircleShape>,
) {
    let mut fills = Vec::new();
    let mut outlined = false;
    let mut circles = Vec::new();
    for shape in painted_shapes(output) {
        match shape {
            egui::Shape::Rect(shape) if shape.rect == rect => {
                if shape.fill != Color32::TRANSPARENT {
                    fills.push((shape.fill, shape.corner_radius));
                }
                outlined |= shape.stroke == theme::BORDER;
            }
            egui::Shape::Circle(circle) => circles.push(circle),
            _ => {}
        }
    }
    (fills, outlined, circles)
}

#[test]
fn the_switch_keeps_its_size_and_colours_in_every_state() {
    for (on, enabled, fill) in [
        (false, true, theme::INACTIVE_LAVENDER),
        (true, true, theme::SELECTED_CYAN),
        (false, false, theme::DISABLED_TOGGLE),
        (true, false, theme::DISABLED_TOGGLE),
    ] {
        let (output, response) = second_frame(true, |ui| {
            let mut on = on;
            switch(ui, &mut on, enabled)
        });
        let state = format!("on={on} enabled={enabled}");
        assert_eq!(response.rect.size(), SWITCH_SIZE, "{state}");
        let (fills, outlined, circles) = pill(&output, response.rect);
        assert_eq!(fills, [(fill, egui::CornerRadius::same(14))], "{state}");
        assert!(outlined, "{state}");
        assert_eq!(circles.len(), 1, "{state}");
        let knob = &circles[0];
        let half = SWITCH_SIZE.y / 2.0;
        let expected_x = if on {
            response.rect.right() - half
        } else {
            response.rect.left() + half
        };
        assert_eq!(
            (knob.center.x, knob.radius),
            (expected_x, half - 3.0),
            "{state}"
        );
        assert_eq!(
            (knob.fill, knob.stroke),
            (Color32::WHITE, Stroke::new(1.0_f32, theme::INK)),
            "{state}"
        );
        assert_eq!(response.sense.senses_click(), enabled, "{state}");
    }
}

#[test]
fn a_click_flips_the_switch_and_a_disabled_one_does_not() {
    for enabled in [true, false] {
        let context = egui::Context::default();
        theme::apply(&context);
        let mut on = false;
        let mut rect = egui::Rect::NOTHING;
        for frame in 0..2 {
            let input = egui::RawInput {
                time: Some(f64::from(frame) * 0.1),
                ..Default::default()
            };
            let _ = context.run_ui(input, |ui| {
                rect = switch(ui, &mut on, enabled).rect;
            });
        }
        let mut changed = false;
        let _ = context.run_ui(click(rect.center(), None, 0.3), |ui| {
            changed = switch(ui, &mut on, enabled).changed();
        });
        assert_eq!((on, changed), (enabled, enabled), "enabled={enabled}");
    }
}
