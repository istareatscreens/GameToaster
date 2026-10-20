pub(in crate::gui::game_launcher_screen) mod calibration;
pub(in crate::gui::game_launcher_screen) mod metrics;

use crate::game_launcher_screen::{ControllerScreenConfig, OrientationProfile};
use crate::gui::common::message::{message, Message};
use crate::gui::game_launcher_screen::display::geometry::{resolve_profile, PixelRect};
use crate::gui::game_launcher_screen::display::monitor::MonitorDescriptor;

pub(in crate::gui::game_launcher_screen) fn resolve(
    monitor: &MonitorDescriptor,
    config: &ControllerScreenConfig,
) -> Result<PixelRect, Message> {
    let monitor_id = monitor.id.key();
    let profile = resolve_profile(config.orientation, monitor.bounds);

    let diameter = match profile {
        OrientationProfile::CabinetPortrait => monitor.bounds.width.min(monitor.bounds.height),
        OrientationProfile::ProductionLandscape => {
            let calibration = config
                .calibration
                .as_ref()
                .ok_or_else(|| message!("controller.calib.needs_config"))?;
            if !calibration::is_current(
                calibration,
                &monitor_id,
                (monitor.bounds.width, monitor.bounds.height),
                monitor.rotation_degrees,
                profile,
            ) {
                return Err(message!("controller.calib.stale"));
            }
            calibration.diameter_px
        }
        OrientationProfile::Auto => unreachable!("resolve_profile never returns Auto"),
    };
    Ok(circle_rect(monitor.bounds, diameter, profile)?)
}

pub(in crate::gui::game_launcher_screen) fn circle_rect(
    monitor: PixelRect,
    diameter_px: u32,
    profile: OrientationProfile,
) -> Result<PixelRect, String> {
    if diameter_px == 0 || diameter_px > monitor.width || diameter_px > monitor.height {
        return Err(format!(
            "circle diameter {diameter_px}px does not fit {}x{} monitor",
            monitor.width, monitor.height
        ));
    }
    let profile = resolve_profile(profile, monitor);
    let x = monitor.x + ((monitor.width - diameter_px) / 2) as i32;
    let y = match profile {
        OrientationProfile::CabinetPortrait => monitor.y + (monitor.height - diameter_px) as i32,
        OrientationProfile::ProductionLandscape => {
            monitor.y + ((monitor.height - diameter_px) / 2) as i32
        }
        OrientationProfile::Auto => unreachable!(),
    };
    Ok(PixelRect {
        x,
        y,
        width: diameter_px,
        height: diameter_px,
    })
}

#[cfg(test)]
pub(in crate::gui::game_launcher_screen) fn fit_only_circle(
    monitor: PixelRect,
    profile: OrientationProfile,
) -> Result<PixelRect, String> {
    circle_rect(monitor, monitor.width.min(monitor.height), profile)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/surface/circle/mod__tests.rs"]
mod tests;
