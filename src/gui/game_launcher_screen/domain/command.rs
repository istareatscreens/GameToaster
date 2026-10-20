use super::circular_game_list::CircularGameList;
use super::model::{ControllerScreenConfig, LauncherXAction};
use crate::launcher_gui::model::GameId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum LauncherCommand {
    Previous,

    Next,

    Launch,

    ToggleMainWindow,

    LauncherX,
}

impl LauncherCommand {
    pub fn enabled(self, screen: &ControllerScreenConfig) -> bool {
        match self {
            Self::LauncherX => screen.ba1_enabled,
            Self::ToggleMainWindow => screen.ba2_enabled,
            Self::Previous | Self::Next | Self::Launch => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ControllerLauncherAction {
    Launch(GameId),
    ToggleMainWindow,
    HideLauncherToTray,
    ExitLauncher,
}

pub(in crate::gui::game_launcher_screen) fn apply(
    command: LauncherCommand,
    screen: &ControllerScreenConfig,
    games: &mut CircularGameList,
    actions: &mut Vec<ControllerLauncherAction>,
) {
    if !command.enabled(screen) {
        return;
    }
    match command {
        LauncherCommand::Previous => {
            games.select_previous();
        }
        LauncherCommand::Next => {
            games.select_next();
        }
        LauncherCommand::Launch => {
            if let Some(game) = games.selected() {
                actions.push(ControllerLauncherAction::Launch(game));
            }
        }
        LauncherCommand::ToggleMainWindow => {
            actions.push(ControllerLauncherAction::ToggleMainWindow)
        }
        LauncherCommand::LauncherX => actions.push(match screen.launcher_x_action {
            LauncherXAction::MinimizeToTray => ControllerLauncherAction::HideLauncherToTray,
            LauncherXAction::ExitLauncher => ControllerLauncherAction::ExitLauncher,
        }),
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/domain/command__tests.rs"]
mod tests;
