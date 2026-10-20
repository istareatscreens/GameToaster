use super::*;

fn applied_styles() -> BTreeMap<TextStyle, FontId> {
    let context = egui::Context::default();
    crate::gui::common::theme::apply(&context);
    context.global_style().text_styles.clone()
}

#[test]
fn every_text_style_maps_onto_the_type_scale() {
    let styles = applied_styles();
    let expected = [
        (TextStyle::Small, 13.0, FontFamily::Proportional),
        (TextStyle::Body, 17.0, FontFamily::Proportional),
        (TextStyle::Button, 17.0, FontFamily::Proportional),
        (TextStyle::Monospace, 15.0, FontFamily::Monospace),
        (TextStyle::Heading, 24.0, FontFamily::Proportional),
    ];
    for (style, size, family) in expected {
        assert_eq!(styles[&style], FontId::new(size, family), "{style:?}");
    }
}

#[test]
fn small_text_is_no_longer_eguis_nine_points() {
    let context = egui::Context::default();
    assert_eq!(TextStyle::Small.resolve(&context.global_style()).size, 9.0);
    crate::gui::common::theme::apply(&context);
    assert_eq!(
        TextStyle::Small.resolve(&context.global_style()).size,
        font_size::SMALL
    );
}

#[test]
fn the_type_scale_is_strictly_increasing() {
    let scale = [
        font_size::TINY,
        font_size::SMALL,
        font_size::CAPTION,
        font_size::COMPACT,
        font_size::MEDIUM,
        font_size::BODY,
        font_size::LARGE,
        font_size::TITLE,
        font_size::ICON,
        font_size::HEADING,
    ];
    assert_eq!(
        scale,
        [12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 20.0, 22.0, 24.0]
    );
    assert!(scale.windows(2).all(|pair| pair[0] < pair[1]));
}
