use super::metrics::chord_width;
use super::*;
use crate::game_launcher_screen::MonitorCalibration;
use crate::gui::game_launcher_screen::display::monitor::StableMonitorId;

#[test]
fn required_fit_only_matrix_matches_research_baselines() {
    let cases = [
        (
            (1080, 1920),
            OrientationProfile::CabinetPortrait,
            (0, 840, 1080),
        ),
        (
            (2160, 4096),
            OrientationProfile::CabinetPortrait,
            (0, 1936, 2160),
        ),
        (
            (1080, 1440),
            OrientationProfile::ProductionLandscape,
            (0, 180, 1080),
        ),
        (
            (1280, 1440),
            OrientationProfile::ProductionLandscape,
            (0, 80, 1280),
        ),
        (
            (1920, 1080),
            OrientationProfile::ProductionLandscape,
            (420, 0, 1080),
        ),
        (
            (4096, 2160),
            OrientationProfile::ProductionLandscape,
            (968, 0, 2160),
        ),
    ];
    for ((width, height), profile, expected) in cases {
        let actual = fit_only_circle(
            PixelRect {
                x: 0,
                y: 0,
                width,
                height,
            },
            profile,
        )
        .unwrap();
        assert_eq!((actual.x, actual.y, actual.width), expected);
        assert_eq!(actual.width, actual.height);
    }
}

#[test]
fn explicit_landscape_overrides_unusual_portrait_aspect() {
    let monitor = PixelRect {
        x: 10,
        y: 20,
        width: 1080,
        height: 1440,
    };
    assert_eq!(
        circle_rect(monitor, 900, OrientationProfile::ProductionLandscape)
            .unwrap()
            .y,
        290
    );
    assert_eq!(
        circle_rect(monitor, 900, OrientationProfile::CabinetPortrait)
            .unwrap()
            .y,
        560
    );
}

#[test]
fn arbitrary_fitting_modes_preserve_bounds_and_chords() {
    for (width, height) in [(900, 1600), (1366, 768), (3440, 1440), (1600, 1600)] {
        for profile in [
            OrientationProfile::CabinetPortrait,
            OrientationProfile::ProductionLandscape,
        ] {
            let monitor = PixelRect {
                x: -200,
                y: 35,
                width,
                height,
            };
            let diameter = width.min(height) * 4 / 5;
            let circle = circle_rect(monitor, diameter, profile).unwrap();
            assert!(circle.x >= monitor.x && circle.y >= monitor.y);
            assert!(circle.x as i64 + circle.width as i64 <= monitor.x as i64 + width as i64);
            assert!(circle.y as i64 + circle.height as i64 <= monitor.y as i64 + height as i64);
            for offset in [-0.45, -0.2, 0.0, 0.2, 0.45] {
                let chord = chord_width(diameter as f32, diameter as f32 * offset);
                assert!(chord > 0.0 && chord <= diameter as f32);
            }
        }
    }
}

fn monitor(width: u32, height: u32) -> MonitorDescriptor {
    MonitorDescriptor {
        index: 0,
        id: StableMonitorId {
            device_name: r"\\.\DISPLAY1".into(),
            device_path: r"\\?\DISPLAY#TST0001#0&0&UID1#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"
                .into(),
            manufacturer: Some("TST".into()),
            product_code: Some(1),
            serial: None,
        },
        friendly_name: "Test".into(),
        bounds: PixelRect {
            x: 100,
            y: -50,
            width,
            height,
        },
        rotation_degrees: 0,
        primary: true,
        dpi_x: 96,
        dpi_y: 96,
        physical_width_mm: None,
        physical_height_mm: None,
        native_pixels: true,
    }
}

#[test]
fn portrait_surface_is_the_bottom_aligned_short_edge_circle_without_calibration() {
    let monitor = monitor(1080, 1920);
    let surface = resolve(&monitor, &ControllerScreenConfig::default()).unwrap();
    assert_eq!(
        surface,
        PixelRect {
            x: 100,
            y: -50 + 840,
            width: 1080,
            height: 1080,
        }
    );
}

#[test]
fn landscape_surface_requires_a_current_calibration() {
    let _locale = crate::gui::common::harness::locale("en");
    let monitor = monitor(1920, 1080);
    let mut config = ControllerScreenConfig::default();

    let missing = resolve(&monitor, &config).unwrap_err();
    assert!(
        !missing.shown.is_empty() && !missing.shown.contains("calibration is stale"),
        "{missing}"
    );

    let mut calibration = MonitorCalibration {
        monitor_id: monitor.id.key(),
        mode_width_px: 1920,
        mode_height_px: 1080,
        rotation_degrees: 0,
        profile: OrientationProfile::ProductionLandscape,
        diameter_px: 900,
    };
    config.calibration = Some(calibration.clone());
    assert_eq!(
        resolve(&monitor, &config).unwrap(),
        PixelRect {
            x: 100 + 510,
            y: -50 + 90,
            width: 900,
            height: 900,
        }
    );

    calibration.mode_width_px = 2560;
    config.calibration = Some(calibration);
    assert!(resolve(&monitor, &config)
        .unwrap_err()
        .shown
        .contains("calibration is stale"));
}
