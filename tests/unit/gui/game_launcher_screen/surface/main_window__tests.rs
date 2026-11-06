use super::*;
use crate::game_launcher_screen::{MonitorCalibration, OrientationProfile};
use crate::gui::game_launcher_screen::display::geometry::PixelRect;
use crate::gui::game_launcher_screen::display::monitor::StableMonitorId;

fn monitor(index: u32, x: i32, width: u32, height: u32, primary: bool) -> MonitorDescriptor {
    MonitorDescriptor {
        index,
        id: StableMonitorId {
            device_name: format!(r"\\.\DISPLAY{}", index + 1),
            device_path: format!(
                r"\\?\DISPLAY#TST000{index}#0&0&UID{index}#{{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}}"
            ),
            manufacturer: Some("TST".into()),
            product_code: Some(index as u16),
            serial: None,
        },
        friendly_name: format!("Test {index}"),
        bounds: PixelRect {
            x,
            y: 0,
            width,
            height,
        },
        rotation_degrees: 0,
        primary,
        dpi_x: 96,
        dpi_y: 96,
        physical_width_mm: None,
        physical_height_mm: None,
        native_pixels: true,
    }
}

fn washing_machine_on(adapter: u32) -> ControllerScreenConfig {
    ControllerScreenConfig {
        enabled: true,
        monitor_adapter: Some(adapter),
        ..ControllerScreenConfig::default()
    }
}

fn monitors() -> Vec<MonitorDescriptor> {
    vec![
        monitor(0, 0, 1920, 1080, true),
        monitor(1, 1920, 1080, 1920, false),
    ]
}

#[test]
fn a_screen_that_is_off_or_uses_another_layout_centres_nothing() {
    let off = ControllerScreenConfig {
        enabled: false,
        ..washing_machine_on(1)
    };
    assert_eq!(circle_on(&monitors(), &off), None);
    let other = ControllerScreenConfig {
        launcher_layout: "another-layout".into(),
        ..washing_machine_on(1)
    };
    assert_eq!(circle_on(&monitors(), &other), None);
}

#[test]
fn a_portrait_monitor_gives_the_bottom_circle_filling_its_width() {
    let circle = circle_on(&monitors(), &washing_machine_on(1)).unwrap();
    assert_eq!(circle.monitor_index, 1);
    assert_eq!(circle.monitor, (1920, 0, 1080, 1920));

    assert_eq!(circle.centre, (1920 + 540, 1920 - 540));
}

#[test]
fn a_calibrated_landscape_monitor_gives_its_centred_circle() {
    let mut config = washing_machine_on(0);
    config.orientation = OrientationProfile::ProductionLandscape;
    config.calibration = Some(MonitorCalibration {
        monitor_id: monitors()[0].id.key(),
        mode_width_px: 1920,
        mode_height_px: 1080,
        rotation_degrees: 0,
        profile: OrientationProfile::ProductionLandscape,
        diameter_px: 900,
    });
    let circle = circle_on(&monitors(), &config).unwrap();
    assert_eq!(circle.monitor_index, 0);
    assert_eq!(circle.centre, (960, 540));
}

#[test]
fn a_landscape_monitor_without_a_current_calibration_centres_nothing() {
    let mut config = washing_machine_on(0);
    config.orientation = OrientationProfile::ProductionLandscape;
    assert_eq!(circle_on(&monitors(), &config), None);
    config.calibration = Some(MonitorCalibration {
        monitor_id: monitors()[0].id.key(),
        mode_width_px: 2560,
        mode_height_px: 1440,
        rotation_degrees: 0,
        profile: OrientationProfile::ProductionLandscape,
        diameter_px: 900,
    });
    assert_eq!(circle_on(&monitors(), &config), None, "a stale calibration");
}

#[test]
fn the_screens_main_display_is_the_primary_monitor() {
    let config = ControllerScreenConfig {
        monitor_adapter: None,
        ..washing_machine_on(0)
    };
    let mut flipped = monitors();
    flipped[0].primary = false;
    flipped[1].primary = true;
    assert_eq!(circle_on(&flipped, &config).unwrap().monitor_index, 1);
}
