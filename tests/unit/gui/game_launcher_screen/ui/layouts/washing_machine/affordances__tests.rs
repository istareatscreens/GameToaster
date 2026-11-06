use super::super::metrics::{ROW_HALF_HEIGHT, ROW_PITCH, UNSELECTED_MAX_WIDTH};
use super::*;

#[test]
fn edge_controls_use_button_ring_angles_and_teardrops_stay_inside() {
    use crate::game_launcher_screen::surface::circle::metrics::CIRCLE_UNITS;

    let controls = edge_controls();
    let angle_of = |action: LauncherCommand| {
        controls
            .iter()
            .find(|(_, _, a)| *a == action)
            .map(|(deg, _, _)| *deg)
            .expect("action is present")
    };
    assert_eq!(angle_of(LauncherCommand::Previous), -157.5);
    assert_eq!(angle_of(LauncherCommand::LauncherX), -67.5);
    assert_eq!(angle_of(LauncherCommand::ToggleMainWindow), -22.5);
    assert_eq!(angle_of(LauncherCommand::Launch), 67.5);
    assert_eq!(angle_of(LauncherCommand::Next), 157.5);

    for diameter in [540.0_f32, 777.0, 1080.0, 2160.0] {
        let radius = diameter / 2.0;
        let center = egui::pos2(radius + 3.0, radius + 3.0);
        let scale = diameter / CIRCLE_UNITS;
        for (angle, _, _) in edge_controls() {
            let (outline, lobe, lobe_r) = teardrop(center, angle, radius, scale);
            for point in &outline {
                let distance = (*point - center).length();
                assert!(
                    distance <= radius + 0.5,
                    "teardrop at {angle} deg escapes the circle: d={distance}, r={radius}"
                );
            }
            assert!(
                (lobe - center).length() + lobe_r <= radius,
                "lobe at {angle} deg is not fully inside the ring"
            );
        }
    }
}

#[test]
fn gear_affordance_maps_to_main_window_toggle_and_follows_ba2() {
    let gear = edge_controls()
        .into_iter()
        .find(|(_, glyph, _)| *glyph == icons::GEAR)
        .expect("a gear affordance exists");
    assert_eq!(gear.2, LauncherCommand::ToggleMainWindow);

    let mut screen = crate::game_launcher_screen::ControllerScreenConfig {
        ba2_enabled: true,
        ..Default::default()
    };
    assert!(LauncherCommand::ToggleMainWindow.enabled(&screen));
    screen.ba2_enabled = false;
    assert!(!LauncherCommand::ToggleMainWindow.enabled(&screen));

    screen.ba1_enabled = false;
    assert!(!LauncherCommand::LauncherX.enabled(&screen));
    screen.ba1_enabled = true;
    assert!(LauncherCommand::LauncherX.enabled(&screen));

    assert!(LauncherCommand::Previous.enabled(&screen));
    assert!(LauncherCommand::Next.enabled(&screen));
    assert!(LauncherCommand::Launch.enabled(&screen));
}

#[test]
fn non_selected_rows_clear_the_edge_affordances() {
    use crate::game_launcher_screen::surface::circle::metrics::{safe_chord_width, CIRCLE_UNITS};

    for diameter in [540.0_f32, 777.0, 1080.0, 2160.0] {
        let radius = diameter / 2.0;
        let scale = diameter / CIRCLE_UNITS;
        let center = egui::pos2(radius + 5.0, radius + 5.0);
        let neighbour_y = ROW_PITCH * scale;
        let safe =
            safe_chord_width(diameter, neighbour_y, ROW_HALF_HEIGHT * scale, 60.0 * scale).max(1.0);
        let unselected_width = safe.min(UNSELECTED_MAX_WIDTH * scale);
        let half_w = unselected_width / 2.0;
        let (row_left, row_right) = (center.x - half_w, center.x + half_w);
        for (angle, _glyph, action) in edge_controls() {
            if !matches!(
                action,
                LauncherCommand::Previous
                    | LauncherCommand::ToggleMainWindow
                    | LauncherCommand::Next
            ) {
                continue;
            }
            let (_outline, lobe_center, lobe_r) = teardrop(center, angle, radius, scale);
            let (lobe_left, lobe_right) = (lobe_center.x - lobe_r, lobe_center.x + lobe_r);
            let clears = row_right <= lobe_left || row_left >= lobe_right;
            assert!(
                clears,
                "non-selected row [{row_left},{row_right}] overlaps {action:?} lobe \
                 [{lobe_left},{lobe_right}] at diameter {diameter}"
            );
        }
    }
}

#[test]
fn the_launching_control_is_the_launch_teardrop_showing_an_x() {
    let launch = edge_controls()
        .into_iter()
        .find(|(_, _, action)| *action == LauncherCommand::Launch)
        .expect("a launch affordance exists");
    assert_eq!(
        launch.1,
        icons::CHECK,
        "the menu's launch button shows the checkmark"
    );

    assert_eq!(
        launching_control(true),
        Some((launch.0, icons::X, LauncherCommand::CancelLaunch))
    );

    assert_eq!(launching_control(false), None);
}

fn click_launching_screen_at(at: egui::Pos2, cancellable: bool) -> Option<LauncherCommand> {
    let context = egui::Context::default();
    let center = egui::pos2(400.0, 400.0);
    let mut command = None;
    let events = [
        egui::Event::PointerMoved(at),
        egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ];
    for event in events {
        let input = egui::RawInput {
            events: vec![event],
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 800.0),
            )),
            ..Default::default()
        };
        let _ = context.run_ui(input, |ui| {
            let painter = ui.painter().clone();
            command = command.or(render_launching_control(
                ui,
                &painter,
                center,
                380.0,
                0.5,
                cancellable,
            ));
        });
    }
    command
}

#[test]
fn clicking_the_x_cancels_and_nothing_else_on_the_launching_screen_does() {
    let (angle, _, _) = launching_control(true).expect("a cancellable launch has the X");
    let (_, lobe, _) = teardrop(egui::pos2(400.0, 400.0), angle, 380.0, 0.5);
    assert_eq!(
        click_launching_screen_at(lobe, true),
        Some(LauncherCommand::CancelLaunch)
    );

    assert_eq!(
        click_launching_screen_at(egui::pos2(400.0, 400.0), true),
        None
    );

    assert_eq!(click_launching_screen_at(lobe, false), None);
}
