use super::*;

fn library() -> CircularGameList {
    let mut games = CircularGameList::default();
    games.rebuild([GameId(1), GameId(2), GameId(3)]);
    games
}

#[test]
fn navigation_and_launch_are_never_gated() {
    let screen = ControllerScreenConfig {
        ba1_enabled: false,
        ba2_enabled: false,
        ..Default::default()
    };
    for command in [
        LauncherCommand::Previous,
        LauncherCommand::Next,
        LauncherCommand::Launch,
    ] {
        assert!(command.enabled(&screen), "{command:?}");
    }
    assert!(!LauncherCommand::LauncherX.enabled(&screen));
    assert!(!LauncherCommand::ToggleMainWindow.enabled(&screen));
}

#[test]
fn each_switch_gates_only_its_own_command() {
    let mut screen = ControllerScreenConfig {
        ba1_enabled: true,
        ba2_enabled: false,
        ..Default::default()
    };
    assert!(LauncherCommand::LauncherX.enabled(&screen));
    assert!(!LauncherCommand::ToggleMainWindow.enabled(&screen));
    screen.ba1_enabled = false;
    screen.ba2_enabled = true;
    assert!(!LauncherCommand::LauncherX.enabled(&screen));
    assert!(LauncherCommand::ToggleMainWindow.enabled(&screen));
}

#[test]
fn apply_moves_the_selection_or_emits_the_app_action() {
    let mut games = library();
    let mut screen = ControllerScreenConfig::default();
    let mut actions = Vec::new();
    apply(LauncherCommand::Next, &screen, &mut games, &mut actions);
    assert_eq!(games.selected(), Some(GameId(2)));
    apply(LauncherCommand::Previous, &screen, &mut games, &mut actions);
    apply(LauncherCommand::Previous, &screen, &mut games, &mut actions);
    assert_eq!(games.selected(), Some(GameId(3)));
    assert!(actions.is_empty(), "navigation never reaches the app shell");

    apply(LauncherCommand::Launch, &screen, &mut games, &mut actions);
    apply(
        LauncherCommand::ToggleMainWindow,
        &screen,
        &mut games,
        &mut actions,
    );
    screen.launcher_x_action = LauncherXAction::MinimizeToTray;
    apply(
        LauncherCommand::LauncherX,
        &screen,
        &mut games,
        &mut actions,
    );
    screen.launcher_x_action = LauncherXAction::ExitLauncher;
    apply(
        LauncherCommand::LauncherX,
        &screen,
        &mut games,
        &mut actions,
    );
    assert_eq!(
        actions,
        vec![
            ControllerLauncherAction::Launch(GameId(3)),
            ControllerLauncherAction::ToggleMainWindow,
            ControllerLauncherAction::HideLauncherToTray,
            ControllerLauncherAction::ExitLauncher,
        ]
    );
}

#[test]
fn a_disabled_command_does_nothing_and_an_empty_library_launches_nothing() {
    let mut games = CircularGameList::default();
    let screen = ControllerScreenConfig {
        ba1_enabled: false,
        ba2_enabled: false,
        ..Default::default()
    };
    let mut actions = Vec::new();
    for command in [
        LauncherCommand::LauncherX,
        LauncherCommand::ToggleMainWindow,
        LauncherCommand::Launch,
        LauncherCommand::Next,
    ] {
        apply(command, &screen, &mut games, &mut actions);
    }
    assert!(actions.is_empty());
    assert_eq!(games.selected(), None);
}

#[test]
fn cancel_launch_is_never_gated_and_becomes_the_app_action() {
    let screen = ControllerScreenConfig {
        ba1_enabled: false,
        ba2_enabled: false,
        ..Default::default()
    };
    assert!(LauncherCommand::CancelLaunch.enabled(&screen));
    let mut games = library();
    let mut actions = Vec::new();
    apply(
        LauncherCommand::CancelLaunch,
        &screen,
        &mut games,
        &mut actions,
    );
    assert_eq!(actions, vec![ControllerLauncherAction::CancelLaunch]);
}

#[test]
fn the_commands_keep_their_affordance_ids() {
    assert_eq!(LauncherCommand::Previous as u8, 0);
    assert_eq!(LauncherCommand::Next as u8, 1);
    assert_eq!(LauncherCommand::Launch as u8, 2);
    assert_eq!(LauncherCommand::ToggleMainWindow as u8, 3);
    assert_eq!(LauncherCommand::LauncherX as u8, 4);
    assert_eq!(LauncherCommand::CancelLaunch as u8, 5);
}
