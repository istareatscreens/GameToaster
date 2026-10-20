use super::*;

fn monitor(name: &str, primary: bool) -> MonitorDescriptor {
    MonitorDescriptor {
        index: 0,
        id: StableMonitorId {
            device_name: name.into(),
            device_path: format!("path-{name}"),
            manufacturer: Some("ABC".into()),
            product_code: Some(1),
            serial: Some(if primary { 1 } else { 2 }),
        },
        friendly_name: name.into(),
        bounds: PixelRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
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

#[test]
fn adapter_index_selects_exactly_and_missing_adapter_falls_back() {
    let mut monitors = vec![monitor("primary", true), monitor("cabinet", false)];
    for (index, monitor) in monitors.iter_mut().enumerate() {
        monitor.index = index as u32;
    }

    assert_eq!(select(&monitors, None, Some(1)).unwrap(), &monitors[1]);

    assert_eq!(select(&monitors, None, Some(9)).unwrap(), &monitors[0]);

    assert_eq!(select(&monitors, None, None).unwrap(), &monitors[0]);
}

#[test]
fn stable_device_path_wins_over_a_renumbered_adapter_index() {
    let mut cabinet = monitor("cabinet", false);
    cabinet.index = 0;
    let mut primary = monitor("primary", true);
    primary.index = 1;
    let monitors = vec![cabinet, primary];

    let saved_id = monitors[0].id.key();
    assert_eq!(
        select(&monitors, Some(saved_id.as_str()), Some(1)).unwrap(),
        &monitors[0]
    );

    assert_eq!(select(&monitors, None, Some(1)).unwrap(), &monitors[1]);
}

#[test]
fn same_physical_monitor_matches_on_device_path_across_renumbering() {
    let saved = r"\\.\DISPLAY5|\\?\DISPLAY#GSM0001#5&abc#{guid}|GSM|1|";
    let now = r"\\.\DISPLAY3|\\?\DISPLAY#GSM0001#5&abc#{guid}|GSM|1|";
    assert!(same_physical_monitor(saved, now));

    let other = r"\\.\DISPLAY3|\\?\DISPLAY#DEL0002#9&xyz#{guid}|DEL|2|";
    assert!(!same_physical_monitor(saved, other));

    assert!(same_physical_monitor(saved, saved));
}

#[test]
fn desktop_dims_swaps_only_on_quarter_turns() {
    assert_eq!(desktop_dims(3840, 2160, 90), (2160, 3840));
    assert_eq!(desktop_dims(3840, 2160, 270), (2160, 3840));

    assert_eq!(desktop_dims(3840, 2160, 0), (3840, 2160));
    assert_eq!(desktop_dims(3840, 2160, 180), (3840, 2160));

    assert_eq!(desktop_dims(1920, 1080, 90), (1080, 1920));
}

#[test]
fn edid_manufacturer_decoding_rejects_invalid_codes() {
    assert_eq!(decode_pnp_manufacturer(0xAC10).as_deref(), Some("DEL"));
    assert_eq!(decode_pnp_manufacturer(0), None);
}

#[test]
#[ignore = "needs a physical display with EDID; run with --include-ignored"]
fn local_inventory_has_unique_stable_paths_and_valid_modes() {
    let monitors = enumerate().unwrap();
    assert!(!monitors.is_empty());
    let mut keys = std::collections::BTreeSet::new();
    for monitor in monitors {
        assert!(
            keys.insert(monitor.id.key()),
            "duplicate stable monitor identity"
        );
        assert!(monitor.bounds.width > 0 && monitor.bounds.height > 0);
        assert!(monitor.dpi_x > 0 && monitor.dpi_y > 0);
        println!(
            "monitor name={} device={} mode={}x{} native_pixels={} rotation={} dpi={}x{} primary={} physical_mm={:?}x{:?}",
            monitor.friendly_name,
            crate::game_launcher_screen::support::diagnostics::redact_device_path(&monitor.id.device_path),
            monitor.bounds.width,
            monitor.bounds.height,
            monitor.native_pixels,
            monitor.rotation_degrees,
            monitor.dpi_x,
            monitor.dpi_y,
            monitor.primary,
            monitor.physical_width_mm,
            monitor.physical_height_mm,
        );
    }
}

#[test]
#[ignore = "needs a physical display with EDID; run with --include-ignored"]
fn auto_calibration_is_sensible_on_real_hardware() {
    use super::super::geometry::resolve_profile;
    use crate::game_launcher_screen::OrientationProfile;
    use crate::gui::game_launcher_screen::surface::circle::calibration::auto_diameter;
    for monitor in enumerate().unwrap() {
        let profile = resolve_profile(OrientationProfile::Auto, monitor.bounds);
        let maximum = monitor.bounds.width.min(monitor.bounds.height);
        let diameter = auto_diameter(
            profile,
            monitor.bounds.width,
            monitor.bounds.height,
            monitor.physical_width_mm,
            monitor.physical_height_mm,
        );
        println!(
            "auto-calib {}: {}x{} profile={:?} -> {:?} (max {})",
            monitor.friendly_name,
            monitor.bounds.width,
            monitor.bounds.height,
            profile,
            diameter,
            maximum,
        );
        match profile {
            OrientationProfile::CabinetPortrait => {
                assert_eq!(
                    diameter.unwrap(),
                    maximum,
                    "portrait must fill the shorter edge"
                );
            }
            OrientationProfile::ProductionLandscape => {
                if let Ok(px) = diameter {
                    assert!(
                        px > 0 && px <= maximum,
                        "landscape diameter must fit the mode"
                    );
                }
            }
            OrientationProfile::Auto => unreachable!(),
        }
    }
}

#[test]
#[ignore = "needs a physical display with EDID; run with --include-ignored"]
fn edid_reader_returns_physical_size_on_real_hardware() {
    let monitors = enumerate().unwrap();
    let mut any = false;
    for monitor in &monitors {
        let edid = super::super::edid::physical_size_mm(&monitor.id.device_path);
        println!(
            "edid {} -> {:?} mm (panel-native, pre-rotation)",
            crate::game_launcher_screen::support::diagnostics::redact_device_path(
                &monitor.id.device_path
            ),
            edid,
        );
        any |= edid.is_some();
    }
    assert!(
        any,
        "EDID reader returned no physical size for any present monitor"
    );
}

#[test]
fn enumeration_is_stable_under_concurrent_callers() {
    fn snapshot() -> Vec<String> {
        enumerate()
            .unwrap_or_else(|error| panic!("enumerate failed: {error}"))
            .iter()
            .map(|monitor| {
                format!(
                    "{}|{}x{}@{},{}|{}|{}",
                    monitor.id.key(),
                    monitor.bounds.width,
                    monitor.bounds.height,
                    monitor.bounds.x,
                    monitor.bounds.y,
                    monitor.rotation_degrees,
                    monitor.native_pixels
                )
            })
            .collect()
    }
    let baseline = snapshot();
    if baseline.is_empty() {
        return;
    }
    let workers: Vec<_> = (0..16)
        .map(|_| {
            let baseline = baseline.clone();
            std::thread::spawn(move || {
                for iteration in 0..200 {
                    let now = snapshot();
                    assert_eq!(now, baseline, "iteration {iteration}");
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

#[test]
fn snapshot_retry_is_bounded_and_reports_the_last_failure() {
    use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER};

    fn run(script: &[Result<u8, SnapshotFailure>]) -> (Result<u8, String>, usize) {
        let mut calls = 0;
        let result = snapshot_with_retry(|| {
            let next = script[calls].clone();
            calls += 1;
            next
        });
        (result, calls)
    }

    let race = SnapshotFailure {
        call: "QueryDisplayConfig",
        code: ERROR_INSUFFICIENT_BUFFER.0,
    };
    let stale = SnapshotFailure {
        call: "DisplayConfigGetDeviceInfo(target name)",
        code: ERROR_INVALID_PARAMETER.0,
    };

    assert_eq!(run(&[Ok(1)]), (Ok(1), 1));

    assert_eq!(
        run(&[Err(race.clone()), Err(stale.clone()), Ok(7)]),
        (Ok(7), 3)
    );

    let (result, calls) = run(&[
        Err(race.clone()),
        Err(race.clone()),
        Err(race),
        Err(stale),
        Ok(9),
    ]);
    assert_eq!(calls, SNAPSHOT_ATTEMPTS);
    let error = result.unwrap_err();
    assert!(error.contains("after 4 attempts"), "{error}");
    assert!(
        error.contains("DisplayConfigGetDeviceInfo(target name) failed with 87"),
        "{error}"
    );
}
