use windows::Win32::System::Threading::{
    CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, PROCESS_CREATION_FLAGS,
};

use super::image::ProgramKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsoleSettings {
    pub show_command_windows: bool,
    pub pause_on_failure: bool,
}

impl ConsoleSettings {
    pub fn hides_consoles(self) -> bool {
        !self.show_command_windows && !self.pause_on_failure
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleWindow {
    Graphical,

    Windowless,

    Hidden,

    Shown,
}

impl ConsoleWindow {
    pub fn creation_flags(self) -> PROCESS_CREATION_FLAGS {
        match self {
            Self::Graphical => PROCESS_CREATION_FLAGS(0),
            Self::Windowless => CREATE_NO_WINDOW,
            Self::Hidden | Self::Shown => CREATE_NEW_CONSOLE,
        }
    }

    pub fn starts_hidden(self) -> bool {
        self == Self::Hidden
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsolePlan {
    pub window: ConsoleWindow,
    pub pause_on_failure: bool,
}

pub fn plan(kind: ProgramKind, settings: ConsoleSettings) -> ConsolePlan {
    let hidden = settings.hides_consoles();
    let (window, pause_on_failure) = match kind {
        ProgramKind::Script if hidden => (ConsoleWindow::Hidden, false),
        ProgramKind::Console if hidden => (ConsoleWindow::Windowless, false),
        ProgramKind::Script | ProgramKind::Console => {
            (ConsoleWindow::Shown, settings.pause_on_failure)
        }

        ProgramKind::Graphical if hidden => (ConsoleWindow::Windowless, false),
        ProgramKind::Graphical => (ConsoleWindow::Graphical, false),
    };
    ConsolePlan {
        window,
        pause_on_failure,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/console_policy__tests.rs"]
mod tests;
