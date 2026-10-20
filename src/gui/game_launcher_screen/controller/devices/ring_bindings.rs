use super::super::framework::buttons::Button;
use super::super::framework::descriptor::ButtonBinding;
use crate::gui::game_launcher_screen::domain::command::LauncherCommand;

pub(super) const RING_BINDINGS: &[ButtonBinding] = &[
    ButtonBinding {
        button: Button::Ba2,
        command: LauncherCommand::LauncherX,
    },
    ButtonBinding {
        button: Button::Ba1,
        command: LauncherCommand::ToggleMainWindow,
    },
    ButtonBinding {
        button: Button::Ba4,
        command: LauncherCommand::Launch,
    },
    ButtonBinding {
        button: Button::Ba6,
        command: LauncherCommand::Next,
    },
    ButtonBinding {
        button: Button::Ba7,
        command: LauncherCommand::Previous,
    },
];

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/devices/ring_bindings__tests.rs"]
mod tests;
