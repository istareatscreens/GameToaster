use crate::game_launcher_screen::ScreenCircle;

pub(super) const MAIN_WINDOW_POINTS: (f32, f32) = (1080.0, 800.0);

pub(super) fn on_circle(
    size: (i32, i32),
    monitor_index: u32,
    circle: Option<ScreenCircle>,
) -> Option<(i32, i32)> {
    let circle = circle.filter(|circle| circle.monitor_index == monitor_index)?;
    let (left, top, width, height) = circle.monitor;
    let x = (circle.centre.0 - size.0 / 2).clamp(left, left + (width - size.0).max(0));
    let y = (circle.centre.1 - size.1 / 2).clamp(top, top + (height - size.1).max(0));
    Some((x, y))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/placement__tests.rs"]
mod tests;
