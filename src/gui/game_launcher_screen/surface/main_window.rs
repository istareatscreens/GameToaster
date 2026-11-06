use super::super::display::monitor::{self, MonitorDescriptor};
use super::super::ui::layouts::WASHING_MACHINE_ID;
use super::circle;
use crate::game_launcher_screen::ControllerScreenConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ScreenCircle {
    pub monitor_index: u32,

    pub monitor: (i32, i32, i32, i32),

    pub centre: (i32, i32),
}

pub(crate) fn main_window_circle(config: &ControllerScreenConfig) -> Option<ScreenCircle> {
    if !centres_main_window(config) {
        return None;
    }
    circle_on(&monitor::enumerate().ok()?, config)
}

fn centres_main_window(config: &ControllerScreenConfig) -> bool {
    config.enabled && config.launcher_layout == WASHING_MACHINE_ID
}

pub(in crate::gui::game_launcher_screen) fn circle_on(
    monitors: &[MonitorDescriptor],
    config: &ControllerScreenConfig,
) -> Option<ScreenCircle> {
    if !centres_main_window(config) {
        return None;
    }
    let wanted_id = config
        .calibration
        .as_ref()
        .map(|calibration| calibration.monitor_id.as_str());
    let monitor = monitor::select(monitors, wanted_id, config.monitor_adapter)?;
    let circle = circle::resolve(monitor, config).ok()?;
    Some(ScreenCircle {
        monitor_index: monitor.index,
        monitor: (
            monitor.bounds.x,
            monitor.bounds.y,
            monitor.bounds.width as i32,
            monitor.bounds.height as i32,
        ),
        centre: (
            circle.x + (circle.width / 2) as i32,
            circle.y + (circle.height / 2) as i32,
        ),
    })
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/surface/main_window__tests.rs"]
mod tests;
