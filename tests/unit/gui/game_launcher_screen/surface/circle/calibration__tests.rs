use super::*;

#[test]
fn portrait_fits_shorter_edge_without_metadata() {
    assert_eq!(
        auto_diameter(OrientationProfile::CabinetPortrait, 2160, 3840, None, None).unwrap(),
        2160
    );
}

#[test]
fn landscape_uses_physical_500mm_and_clamps() {
    let d = auto_diameter(
        OrientationProfile::ProductionLandscape,
        1920,
        1080,
        Some(531),
        Some(299),
    )
    .unwrap();
    assert_eq!(d, 1080);

    let big = auto_diameter(
        OrientationProfile::ProductionLandscape,
        3840,
        2160,
        Some(1600),
        Some(900),
    )
    .unwrap();

    assert_eq!(big, 1200);
}

#[test]
fn landscape_without_metadata_is_a_typed_manual_fallback() {
    let _locale = crate::gui::common::harness::locale("en");
    let err = auto_diameter(
        OrientationProfile::ProductionLandscape,
        1920,
        1080,
        None,
        None,
    )
    .unwrap_err();
    assert!(err.contains("calibrate manually"));
}

#[test]
fn inconsistent_density_is_rejected() {
    let _locale = crate::gui::common::harness::locale("en");

    let err = auto_diameter(
        OrientationProfile::ProductionLandscape,
        1920,
        1080,
        Some(500),
        Some(500),
    )
    .unwrap_err();
    assert!(err.contains("inconsistent"));
}

#[test]
fn build_enforces_fit_and_is_current_tracks_mode() {
    assert!(build(
        "monitor".into(),
        (1080, 1920),
        0,
        OrientationProfile::CabinetPortrait,
        1200,
    )
    .is_err());
    let valid = build(
        "monitor".into(),
        (1080, 1920),
        0,
        OrientationProfile::CabinetPortrait,
        1080,
    )
    .unwrap();
    assert!(is_current(
        &valid,
        "monitor",
        (1080, 1920),
        0,
        OrientationProfile::CabinetPortrait,
    ));

    assert!(!is_current(
        &valid,
        "monitor",
        (1080, 1920),
        90,
        OrientationProfile::CabinetPortrait,
    ));
}
