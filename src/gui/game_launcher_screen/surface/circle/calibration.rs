use rust_i18n::t;

use crate::game_launcher_screen::{MonitorCalibration, OrientationProfile};

pub(super) const TARGET_DIAMETER_MM: f64 = 500.0;

const MIN_PHYSICAL_MM: u32 = 50;
const MAX_PHYSICAL_MM: u32 = 5000;

const DENSITY_TOLERANCE: f64 = 0.06;

pub(in crate::gui::game_launcher_screen) fn auto_diameter(
    profile: OrientationProfile,
    native_width: u32,
    native_height: u32,
    physical_width_mm: Option<u32>,
    physical_height_mm: Option<u32>,
) -> Result<u32, String> {
    let maximum = native_width.min(native_height);
    if maximum == 0 {
        return Err("monitor reports an empty native resolution".into());
    }
    match profile {
        OrientationProfile::CabinetPortrait => Ok(maximum),
        OrientationProfile::ProductionLandscape => {
            let (density, _) = plausible_density(
                native_width,
                native_height,
                physical_width_mm,
                physical_height_mm,
            )?;
            let diameter = (TARGET_DIAMETER_MM * density).round();
            if !diameter.is_finite() || diameter <= 0.0 {
                return Err("physical metadata produced an invalid diameter".into());
            }
            Ok((diameter as u32).min(maximum))
        }
        OrientationProfile::Auto => Err("auto-calibration requires a resolved orientation".into()),
    }
}

fn plausible_density(
    native_width: u32,
    native_height: u32,
    physical_width_mm: Option<u32>,
    physical_height_mm: Option<u32>,
) -> Result<(f64, f64), String> {
    let width_mm = physical_width_mm
        .filter(|mm| is_plausible(*mm))
        .ok_or_else(|| t!("controller.calib.size_unknown").to_string())?;
    let height_mm = physical_height_mm
        .filter(|mm| is_plausible(*mm))
        .ok_or_else(|| t!("controller.calib.size_unknown").to_string())?;
    let density_x = native_width as f64 / width_mm as f64;
    let density_y = native_height as f64 / height_mm as f64;
    let relative = (density_x - density_y).abs() / density_x.max(density_y).max(f64::EPSILON);
    if relative > DENSITY_TOLERANCE {
        return Err(t!("controller.calib.density_inconsistent").to_string());
    }
    Ok(((density_x + density_y) / 2.0, relative))
}

fn is_plausible(mm: u32) -> bool {
    (MIN_PHYSICAL_MM..=MAX_PHYSICAL_MM).contains(&mm)
}

pub(in crate::gui::game_launcher_screen) fn build(
    monitor_id: String,
    mode: (u32, u32),
    rotation_degrees: u16,
    profile: OrientationProfile,
    diameter_px: u32,
) -> Result<MonitorCalibration, String> {
    if monitor_id.trim().is_empty()
        || diameter_px == 0
        || diameter_px > mode.0
        || diameter_px > mode.1
    {
        return Err(t!("controller.calib.circle_does_not_fit").to_string());
    }
    Ok(MonitorCalibration {
        monitor_id,
        mode_width_px: mode.0,
        mode_height_px: mode.1,
        rotation_degrees,
        profile,
        diameter_px,
    })
}

pub(in crate::gui::game_launcher_screen) fn is_current(
    calibration: &MonitorCalibration,
    monitor_id: &str,
    mode: (u32, u32),
    rotation_degrees: u16,
    profile: OrientationProfile,
) -> bool {
    crate::gui::game_launcher_screen::display::monitor::same_physical_monitor(
        &calibration.monitor_id,
        monitor_id,
    ) && (calibration.mode_width_px, calibration.mode_height_px) == mode
        && calibration.rotation_degrees == rotation_degrees
        && calibration.profile == profile
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/surface/circle/calibration__tests.rs"]
mod tests;
