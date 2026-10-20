use super::*;
use crate::game_launcher_screen::{
    ControllerLauncherAction, ControllerScreenConfig, LauncherXAction,
};
use crate::gui::game_launcher_screen::domain::circular_game_list::CircularGameList;
use crate::gui::game_launcher_screen::domain::command::apply;
use crate::launcher_gui::model::GameId;

fn press(
    button: Button,
    config: &ControllerScreenConfig,
    games: &mut CircularGameList,
    actions: &mut Vec<ControllerLauncherAction>,
) {
    if let Some(binding) = RING_BINDINGS
        .iter()
        .find(|binding| binding.button == button)
    {
        apply(binding.command, config, games, actions);
    }
}

#[test]
fn the_ring_binds_exactly_the_five_cabinet_controls() {
    let expected = [
        (Button::Ba2, LauncherCommand::LauncherX),
        (Button::Ba1, LauncherCommand::ToggleMainWindow),
        (Button::Ba4, LauncherCommand::Launch),
        (Button::Ba6, LauncherCommand::Next),
        (Button::Ba7, LauncherCommand::Previous),
    ];
    let actual: Vec<(Button, LauncherCommand)> = RING_BINDINGS
        .iter()
        .map(|binding| (binding.button, binding.command))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn every_ring_device_routes_through_the_ring_bindings() {
    for driver in crate::game_launcher_screen::controller::AdapterRegistry::with_default_adapters()
        .descriptors()
    {
        assert_eq!(driver.bindings, RING_BINDINGS, "{}", driver.id);
    }
}

#[test]
fn button_actions_honor_policy_and_circular_navigation() {
    let mut games = CircularGameList::default();
    games.rebuild([GameId(1), GameId(2), GameId(3)]);
    let mut config = ControllerScreenConfig::default();
    let mut actions = Vec::new();

    press(Button::Ba7, &config, &mut games, &mut actions);
    assert_eq!(games.selected(), Some(GameId(3)));
    press(Button::Ba6, &config, &mut games, &mut actions);
    assert_eq!(games.selected(), Some(GameId(1)));
    press(Button::Ba4, &config, &mut games, &mut actions);
    assert_eq!(
        actions.pop(),
        Some(ControllerLauncherAction::Launch(GameId(1)))
    );

    config.ba1_enabled = false;
    config.ba2_enabled = false;
    press(Button::Ba1, &config, &mut games, &mut actions);
    press(Button::Ba2, &config, &mut games, &mut actions);
    assert!(actions.is_empty());

    config.ba1_enabled = true;
    config.launcher_x_action = LauncherXAction::ExitLauncher;
    press(Button::Ba2, &config, &mut games, &mut actions);
    assert_eq!(actions, vec![ControllerLauncherAction::ExitLauncher]);
    actions.clear();

    config.ba2_enabled = true;
    press(Button::Ba1, &config, &mut games, &mut actions);
    assert_eq!(actions, vec![ControllerLauncherAction::ToggleMainWindow]);
}
