use super::*;
use crate::game_launcher_screen::{MonitorCalibration, OrientationProfile};
use crate::gui::game_launcher_screen::display;

#[test]
fn configured_display_available_is_strict_about_the_configured_monitor() {
    let monitors = display::monitor::enumerate().unwrap();
    let Some(monitor) = monitors.first() else {
        return;
    };
    let mut config = ControllerScreenConfig::default();

    assert!(configured_display_available(&config));

    config.monitor_adapter = Some(monitor.index);
    assert!(configured_display_available(&config));
    config.calibration = Some(MonitorCalibration {
        monitor_id: "missing-device|missing-path|unknown||".into(),
        mode_width_px: monitor.bounds.width,
        mode_height_px: monitor.bounds.height,
        rotation_degrees: monitor.rotation_degrees,
        profile: OrientationProfile::ProductionLandscape,
        diameter_px: monitor.bounds.width.min(monitor.bounds.height),
    });
    assert!(!configured_display_available(&config));
    config.calibration = None;

    config.monitor_adapter = Some(u32::MAX);
    assert!(!configured_display_available(&config));
}

#[test]
fn an_absent_configured_monitor_is_not_active_while_its_return_is_enabled() {
    let _locale = crate::gui::common::harness::locale("en");

    if display::monitor::enumerate().is_err() {
        return;
    }
    let config = ControllerScreenConfig {
        enabled: true,
        reopen_on_monitor_return: true,
        monitor_adapter: Some(u32::MAX),
        ..Default::default()
    };
    assert_eq!(
        resolve(&config).unwrap_err().shown,
        "the configured game launcher monitor is not active"
    );
}
