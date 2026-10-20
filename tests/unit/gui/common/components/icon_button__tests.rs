use super::*;
use crate::gui::common::harness::{painted_shapes, second_frame};

fn tile_look(output: &egui::FullOutput, rect: egui::Rect) -> (Color32, bool, Color32) {
    let shapes = painted_shapes(output);
    let fill = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.rect == rect && rect_shape.fill != Color32::TRANSPARENT =>
            {
                assert_eq!(rect_shape.corner_radius, egui::CornerRadius::same(7));
                Some(rect_shape.fill)
            }
            _ => None,
        })
        .expect("the tile paints its fill");
    let outlined = shapes.iter().any(|shape| {
        matches!(shape, egui::Shape::Rect(rect_shape)
            if rect_shape.rect == rect
                && rect_shape.stroke == theme::BORDER
                && rect_shape.corner_radius == egui::CornerRadius::same(7))
    });
    let ink = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.text() == "G" => {
                assert_eq!(
                    text.galley.job.sections[0].format.font_id.size,
                    theme::font_size::ICON
                );
                Some(text.galley.job.sections[0].format.color)
            }
            _ => None,
        })
        .expect("the tile paints its glyph");
    (fill, outlined, ink)
}

#[test]
fn a_tile_keeps_its_size_radius_and_colours_in_every_state() {
    let fill = theme::RAINBOW_SKY;
    let half = fill.gamma_multiply(0.5);
    for (tone, enabled, hovered, background, ink) in [
        (TileTone::Plain, true, false, fill, theme::INK),
        (
            TileTone::Plain,
            true,
            true,
            fill.gamma_multiply(1.1),
            theme::INK,
        ),
        (TileTone::Plain, false, false, half, theme::MUTED),
        (TileTone::Plain, false, true, half, theme::MUTED),
        (TileTone::Danger, true, false, fill, theme::ERROR),
        (
            TileTone::Danger,
            true,
            true,
            theme::CLOSE_HOVER,
            theme::ERROR,
        ),
        (TileTone::Danger, false, true, half, theme::MUTED),
    ] {
        let (output, response) =
            second_frame(hovered, |ui| tile_button(ui, "G", fill, tone, enabled));
        assert_eq!(response.rect.size(), TILE_SIZE);
        assert_eq!(
            tile_look(&output, response.rect),
            (background, true, ink),
            "{tone:?} enabled={enabled} hovered={hovered}"
        );
        assert_eq!(
            response.sense.senses_click(),
            enabled,
            "a disabled tile senses hover only"
        );
    }
}

#[test]
fn the_close_button_turns_red_on_hover_and_keeps_its_ink_cross() {
    let size = Vec2::new(46.0, 38.0);
    for hovered in [false, true] {
        let (output, response) = second_frame(hovered, |ui| window_close_button(ui, size));
        assert_eq!(response.rect.size(), size);
        assert!(response.sense.senses_click());
        let shapes = painted_shapes(&output);
        let red = shapes.iter().any(|shape| {
            matches!(shape, egui::Shape::Rect(rect_shape)
                if rect_shape.rect == response.rect && rect_shape.fill == theme::CLOSE_HOVER)
        });
        assert_eq!(red, hovered);
        let cross = shapes.iter().any(|shape| {
            matches!(shape, egui::Shape::Text(text)
                if text.galley.text() == icons::X
                    && text.galley.job.sections[0].format.color == theme::INK
                    && text.galley.job.sections[0].format.font_id.size == theme::font_size::LARGE)
        });
        assert!(cross, "the cross is an ink glyph in the large size");
    }
}
