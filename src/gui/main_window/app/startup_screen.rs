#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StartupScreenAction {
    Nothing,

    OpenInTray,

    OpenForUser { controller_only: bool },

    ArmMonitorReturn,

    RevealMonitorMissing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct StartupScreenInputs {
    pub enabled: bool,

    pub started_in_tray: bool,

    pub open_on_os_startup: bool,

    pub launch_on_start: bool,

    pub controller_only_start: bool,

    pub monitor_return_configured: bool,
}

pub(super) fn decide(
    inputs: StartupScreenInputs,
    display_available: impl FnOnce() -> bool,
) -> StartupScreenAction {
    if !inputs.enabled {
        return StartupScreenAction::Nothing;
    }
    if inputs.started_in_tray {
        if !inputs.open_on_os_startup {
            return StartupScreenAction::Nothing;
        }
        if display_available() {
            return StartupScreenAction::OpenInTray;
        }

        if inputs.monitor_return_configured {
            return StartupScreenAction::ArmMonitorReturn;
        }
        return StartupScreenAction::RevealMonitorMissing;
    }
    if inputs.launch_on_start {
        return StartupScreenAction::OpenForUser {
            controller_only: inputs.controller_only_start,
        };
    }
    StartupScreenAction::Nothing
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/startup_screen__tests.rs"]
mod tests;
