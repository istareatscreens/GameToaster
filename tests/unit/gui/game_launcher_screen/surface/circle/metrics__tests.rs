use super::*;

#[test]
fn canonical_metrics_scale_uniformly() {
    assert_eq!(canonical_scale(2160), 2.0);
    assert_eq!(ARTWORK_UNITS * canonical_scale(540), 48.0);
    assert!((chord_width(1080.0, 0.0) - 1080.0).abs() < 0.001);
    assert!(circle_area(1080.0) > 900_000.0);
}

#[test]
fn edge_aware_chords_keep_every_content_corner_inside_the_circle() {
    for (diameter, y, half_height, inset) in [
        (1080.0, 0.0, 48.0, 45.0),
        (1080.0, 318.0, 48.0, 45.0),
        (2160.0, 636.0, 96.0, 90.0),
        (777.0, -225.0, 34.0, 28.0),
    ] {
        let width = safe_chord_width(diameter, y, half_height, inset);
        let radius = diameter / 2.0;
        let corner_x = width / 2.0;
        let corner_y = y.abs() + half_height;
        assert!(corner_x * corner_x + corner_y * corner_y <= radius * radius);
    }
}

#[test]
fn canonical_artwork_box_is_exactly_ninety_six_units() {
    assert_eq!(
        crate::game_launcher_screen::surface::circle::metrics::ARTWORK_UNITS,
        96.0
    );
}
