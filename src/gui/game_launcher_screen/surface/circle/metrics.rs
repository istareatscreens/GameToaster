#[cfg(test)]
use std::f32::consts::PI;

pub(in crate::gui::game_launcher_screen) const CIRCLE_UNITS: f32 = 1080.0;
pub(in crate::gui::game_launcher_screen) const ARTWORK_UNITS: f32 = 96.0;

#[cfg(test)]
pub(in crate::gui::game_launcher_screen) fn canonical_scale(diameter_px: u32) -> f32 {
    diameter_px as f32 / CIRCLE_UNITS
}

pub(in crate::gui::game_launcher_screen) fn chord_width(
    diameter_px: f32,
    row_center_from_circle_center: f32,
) -> f32 {
    let radius = diameter_px / 2.0;
    let y = row_center_from_circle_center.abs().min(radius);
    2.0 * (radius * radius - y * y).max(0.0).sqrt()
}

pub(in crate::gui::game_launcher_screen) fn safe_chord_width(
    diameter_px: f32,
    row_center_from_circle_center: f32,
    half_height: f32,
    horizontal_inset: f32,
) -> f32 {
    let outer_edge = row_center_from_circle_center.abs() + half_height.max(0.0);
    (chord_width(diameter_px, outer_edge) - horizontal_inset.max(0.0) * 2.0).max(0.0)
}

#[cfg(test)]
pub(super) fn circle_area(diameter_px: f32) -> f32 {
    PI * (diameter_px / 2.0).powi(2)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/surface/circle/metrics__tests.rs"]
mod tests;
