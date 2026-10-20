use super::*;

const HIDDEN: ConsoleSettings = ConsoleSettings {
    show_command_windows: false,
    pause_on_failure: false,
};
const SHOWN: ConsoleSettings = ConsoleSettings {
    show_command_windows: true,
    pause_on_failure: false,
};
const PAUSE: ConsoleSettings = ConsoleSettings {
    show_command_windows: false,
    pause_on_failure: true,
};
const SHOWN_AND_PAUSE: ConsoleSettings = ConsoleSettings {
    show_command_windows: true,
    pause_on_failure: true,
};

fn expect(kind: ProgramKind, settings: ConsoleSettings, window: ConsoleWindow, pause: bool) {
    assert_eq!(
        plan(kind, settings),
        ConsolePlan {
            window,
            pause_on_failure: pause
        },
        "{kind:?} {settings:?}"
    );
}

#[test]
fn consoles_are_hidden_only_while_command_windows_are_hidden_and_pause_is_off() {
    assert!(HIDDEN.hides_consoles());
    assert!(!SHOWN.hides_consoles());
    assert!(!PAUSE.hides_consoles());
    assert!(!SHOWN_AND_PAUSE.hides_consoles());
}

#[test]
fn a_script_runs_in_a_hidden_window_or_a_shown_one_and_pauses_when_asked() {
    expect(ProgramKind::Script, HIDDEN, ConsoleWindow::Hidden, false);
    expect(ProgramKind::Script, SHOWN, ConsoleWindow::Shown, false);
    expect(ProgramKind::Script, PAUSE, ConsoleWindow::Shown, true);
    expect(
        ProgramKind::Script,
        SHOWN_AND_PAUSE,
        ConsoleWindow::Shown,
        true,
    );
}

#[test]
fn a_console_program_has_no_window_or_a_shown_one_and_pauses_when_asked() {
    expect(
        ProgramKind::Console,
        HIDDEN,
        ConsoleWindow::Windowless,
        false,
    );
    expect(ProgramKind::Console, SHOWN, ConsoleWindow::Shown, false);
    expect(ProgramKind::Console, PAUSE, ConsoleWindow::Shown, true);
    expect(
        ProgramKind::Console,
        SHOWN_AND_PAUSE,
        ConsoleWindow::Shown,
        true,
    );
}

#[test]
fn a_graphical_program_starts_as_before_and_never_pauses() {
    expect(
        ProgramKind::Graphical,
        HIDDEN,
        ConsoleWindow::Windowless,
        false,
    );
    expect(
        ProgramKind::Graphical,
        SHOWN,
        ConsoleWindow::Graphical,
        false,
    );
    expect(
        ProgramKind::Graphical,
        PAUSE,
        ConsoleWindow::Graphical,
        false,
    );
    expect(
        ProgramKind::Graphical,
        SHOWN_AND_PAUSE,
        ConsoleWindow::Graphical,
        false,
    );
}

#[test]
fn each_console_has_its_creation_flags_and_only_a_hidden_window_starts_hidden() {
    assert_eq!(ConsoleWindow::Graphical.creation_flags().0, 0);
    assert_eq!(ConsoleWindow::Windowless.creation_flags(), CREATE_NO_WINDOW);
    assert_eq!(ConsoleWindow::Hidden.creation_flags(), CREATE_NEW_CONSOLE);
    assert_eq!(ConsoleWindow::Shown.creation_flags(), CREATE_NEW_CONSOLE);
    assert!(ConsoleWindow::Hidden.starts_hidden());
    for window in [
        ConsoleWindow::Graphical,
        ConsoleWindow::Windowless,
        ConsoleWindow::Shown,
    ] {
        assert!(!window.starts_hidden(), "{window:?}");
    }
}
