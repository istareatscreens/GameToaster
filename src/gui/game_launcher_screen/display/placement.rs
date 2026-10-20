use eframe::egui;

use super::geometry::PixelRect;

const TOLERANCE_POINTS: f32 = 1.5;

pub(in crate::gui::game_launcher_screen) const MAX_ATTEMPTS: u32 = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct PlacementTarget {
    pub position: egui::Pos2,
    pub size: egui::Vec2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum Placement {
    Issue,

    Converged,

    GiveUp,
}

pub(in crate::gui::game_launcher_screen) fn target_in_points(
    bounds: PixelRect,
    pixels_per_point: f32,
) -> PlacementTarget {
    let scale = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    PlacementTarget {
        position: egui::pos2(bounds.x as f32 / scale, bounds.y as f32 / scale),
        size: egui::vec2(bounds.width as f32 / scale, bounds.height as f32 / scale),
    }
}

pub(in crate::gui::game_launcher_screen) fn decide(
    target: PlacementTarget,
    reported: Option<egui::Rect>,
    attempts: u32,
) -> Placement {
    if let Some(reported) = reported {
        if matches(target, reported) {
            return Placement::Converged;
        }
    }
    if attempts >= MAX_ATTEMPTS {
        return Placement::GiveUp;
    }
    Placement::Issue
}

fn matches(target: PlacementTarget, reported: egui::Rect) -> bool {
    close(reported.min.x, target.position.x)
        && close(reported.min.y, target.position.y)
        && close(reported.width(), target.size.x)
        && close(reported.height(), target.size.y)
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() <= TOLERANCE_POINTS
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/display/placement__tests.rs"]
mod tests;
