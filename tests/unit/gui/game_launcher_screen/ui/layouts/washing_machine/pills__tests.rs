use super::super::metrics::{ROW_HEIGHT, ROW_PITCH};
use super::*;

#[test]
fn loading_status_sits_in_the_top_cap_inside_the_circle() {
    use crate::game_launcher_screen::surface::circle::metrics::CIRCLE_UNITS;

    for diameter in [540.0_f32, 777.0, 1080.0, 2160.0] {
        let radius = diameter / 2.0;
        let scale = diameter / CIRCLE_UNITS;
        let status_offset = LOADING_STATUS_TOP_OFFSET * scale;
        let top_row_edge = (2.0 * ROW_PITCH + ROW_HEIGHT / 2.0) * scale;
        assert!(
            status_offset >= top_row_edge,
            "status must sit above the top row at diameter {diameter}"
        );
        let font = (28.0 * scale).max(14.0);
        assert!(
            status_offset + font / 2.0 < radius - (10.0 * scale).max(2.0),
            "status must stay inside the top mint ring at diameter {diameter}"
        );
    }
}

#[test]
fn credit_readout_sits_in_the_bottom_cap_inside_the_circle() {
    use crate::game_launcher_screen::surface::circle::metrics::CIRCLE_UNITS;

    for diameter in [540.0_f32, 777.0, 1080.0, 2160.0] {
        let radius = diameter / 2.0;
        let scale = diameter / CIRCLE_UNITS;
        let readout_offset = CREDIT_READOUT_BOTTOM_OFFSET * scale;
        let bottom_row_edge = (2.0 * ROW_PITCH + ROW_HEIGHT / 2.0) * scale;
        assert!(
            readout_offset >= bottom_row_edge,
            "credit readout must sit below the bottom row at diameter {diameter}"
        );
        let font = (28.0 * scale).max(14.0);
        assert!(
            readout_offset + font / 2.0 < radius - (10.0 * scale).max(2.0),
            "credit readout must stay inside the bottom mint ring at diameter {diameter}"
        );

        let dy = CREDIT_READOUT_BOTTOM_OFFSET * scale;
        let chord_half = (radius * radius - dy * dy).max(0.0).sqrt();
        assert!(
            chord_half > 20.0 * scale,
            "the bottom cap must leave room for the pill at diameter {diameter}"
        );
    }
}
