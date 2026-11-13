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
        (Button::Ba1, LauncherCommand::LauncherX),
        (Button::Ba2, LauncherCommand::ToggleMainWindow),
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
    press(Button::Ba1, &config, &mut games, &mut actions);
    assert_eq!(actions, vec![ControllerLauncherAction::ExitLauncher]);
    actions.clear();

    config.ba2_enabled = true;
    press(Button::Ba2, &config, &mut games, &mut actions);
    assert_eq!(actions, vec![ControllerLauncherAction::ToggleMainWindow]);
}

fn commands(word: crate::game_launcher_screen::controller::framework::buttons::ButtonWord) -> u8 {
    RING_BINDINGS
        .iter()
        .filter(|binding| word.contains(binding.button))
        .fold(0, |held, binding| held | command_bit(binding.command))
}

fn command_bit(command: LauncherCommand) -> u8 {
    match command {
        LauncherCommand::LauncherX => 1,
        LauncherCommand::ToggleMainWindow => 2,
        LauncherCommand::Launch => 4,
        LauncherCommand::Next => 8,
        LauncherCommand::Previous => 16,
        LauncherCommand::CancelLaunch => 32,
    }
}

#[test]
fn the_io4_ring_performs_the_commands_it_always_did() {
    use super::super::io4_report::{Io4ReportDecoder, REPORT_SIZE};
    use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

    let mut report = [0u8; REPORT_SIZE];
    report[0] = 0x01;
    for first in 0..=255u8 {
        for second in 0..=255u8 {
            report[29] = first;
            report[30] = second;
            let mut golden = 0;
            if first & 0x04 == 0 {
                golden |= command_bit(LauncherCommand::LauncherX);
            }
            if first & 0x08 == 0 {
                golden |= command_bit(LauncherCommand::ToggleMainWindow);
            }
            if second & 0x80 == 0 {
                golden |= command_bit(LauncherCommand::Launch);
            }
            if second & 0x20 == 0 {
                golden |= command_bit(LauncherCommand::Next);
            }
            if second & 0x10 == 0 {
                golden |= command_bit(LauncherCommand::Previous);
            }
            let word = Io4ReportDecoder.decode(&report).unwrap();
            assert_eq!(commands(word), golden, "bytes {first:#04x} {second:#04x}");
        }
    }
}

#[test]
fn a_yuan_hid_puts_each_action_under_the_same_button_as_an_io4_ring() {
    use super::super::io4_report::{Io4ReportDecoder, REPORT_SIZE};
    use super::super::yuan_hid_report::YuanHidReportDecoder;
    use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

    let same_button = [
        (29, 0x04, 5),
        (29, 0x08, 4),
        (29, 0x01, 3),
        (30, 0x80, 2),
        (30, 0x40, 9),
        (30, 0x20, 8),
        (30, 0x10, 7),
        (30, 0x08, 6),
    ];
    for (at, mask, field) in same_button {
        let mut board = [0u8; REPORT_SIZE];
        board[0] = 0x01;
        board[29] = 0x0D;
        board[30] = 0xF8;
        board[at] &= !mask;
        let mut ring = [0u8; 33];
        ring[1] = 0x02;
        ring[field] = 0x01;
        let from_board = Io4ReportDecoder.decode(&board).unwrap();
        let from_ring = YuanHidReportDecoder.decode(&ring).unwrap();
        assert_eq!(from_board, from_ring, "field {field}");
        assert_eq!(commands(from_board), commands(from_ring), "field {field}");
    }
}
