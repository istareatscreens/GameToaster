use super::*;

fn bounds() -> PixelRect {
    PixelRect {
        x: 1920,
        y: 0,
        width: 2160,
        height: 3840,
    }
}

fn rect(target: PlacementTarget) -> egui::Rect {
    egui::Rect::from_min_size(target.position, target.size)
}

#[test]
fn target_is_expressed_in_points_for_the_windows_current_scale() {
    let at_100 = target_in_points(bounds(), 1.0);
    assert_eq!(at_100.position, egui::pos2(1920.0, 0.0));
    assert_eq!(at_100.size, egui::vec2(2160.0, 3840.0));

    let at_150 = target_in_points(bounds(), 1.5);
    assert_eq!(at_150.position, egui::pos2(1280.0, 0.0));
    assert_eq!(at_150.size, egui::vec2(1440.0, 2560.0));
}

#[test]
fn a_nonsense_scale_falls_back_to_one_point_per_pixel() {
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let target = target_in_points(bounds(), scale);
        assert!(target.position.x.is_finite(), "scale {scale}");
        assert_eq!(target.position, egui::pos2(1920.0, 0.0), "scale {scale}");
        assert_eq!(target.size, egui::vec2(2160.0, 3840.0), "scale {scale}");
    }
}

#[test]
fn an_unreported_rectangle_is_never_treated_as_converged() {
    let target = target_in_points(bounds(), 1.0);
    assert_eq!(decide(target, None, 0), Placement::Issue);
    assert_eq!(decide(target, None, MAX_ATTEMPTS - 1), Placement::Issue);
    assert_eq!(decide(target, None, MAX_ATTEMPTS), Placement::GiveUp);
}

#[test]
fn a_window_already_on_target_converges_without_a_command() {
    let target = target_in_points(bounds(), 1.0);
    assert_eq!(decide(target, Some(rect(target)), 0), Placement::Converged);

    assert_eq!(
        decide(target, Some(rect(target)), MAX_ATTEMPTS + 5),
        Placement::Converged
    );
}

#[test]
fn a_displaced_window_is_issued_commands_until_it_reports_the_target() {
    let target = target_in_points(bounds(), 1.0);

    let displaced = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), target.size);
    assert_eq!(decide(target, Some(displaced), 0), Placement::Issue);
    assert_eq!(decide(target, Some(displaced), 1), Placement::Issue);
    assert_eq!(
        decide(target, Some(displaced), MAX_ATTEMPTS),
        Placement::GiveUp
    );
}

#[test]
fn a_cross_dpi_move_converges_on_the_scale_it_lands_at() {
    let first = target_in_points(bounds(), 1.0);
    let on_primary = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), first.size);
    assert_eq!(decide(first, Some(on_primary), 0), Placement::Issue);

    let second = target_in_points(bounds(), 1.5);
    assert_eq!(decide(second, Some(rect(second)), 1), Placement::Converged);

    assert_eq!(decide(second, Some(rect(first)), 1), Placement::Issue);
}

#[test]
fn sub_point_rounding_is_tolerated() {
    let target = target_in_points(bounds(), 1.5);
    let rounded = egui::Rect::from_min_size(
        egui::pos2(target.position.x + 0.7, target.position.y - 0.4),
        egui::vec2(target.size.x + 0.5, target.size.y - 0.6),
    );
    assert_eq!(decide(target, Some(rounded), 0), Placement::Converged);

    let off_by_a_lot = egui::Rect::from_min_size(
        egui::pos2(target.position.x + 40.0, target.position.y),
        target.size,
    );
    assert_eq!(decide(target, Some(off_by_a_lot), 0), Placement::Issue);
}
