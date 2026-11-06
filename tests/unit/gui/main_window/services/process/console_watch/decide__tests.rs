use super::*;

fn seen(window: isize, visible: bool) -> Seen {
    Seen {
        window,
        visible,
        minimized: false,
        paused: false,
    }
}

#[test]
fn a_shown_console_of_a_session_is_hidden_and_remembered() {
    let mut decisions = Decisions::default();
    assert_eq!(decisions.member(&seen(1, true), true), Action::Hide);
    let minimized = Seen {
        minimized: true,
        ..seen(2, true)
    };
    assert_eq!(decisions.member(&minimized, true), Action::Hide);
    assert_eq!(
        decisions.stop_hiding(|_| true),
        vec![(1, false), (2, true)],
        "shown again as they were, minimized when they were"
    );
    assert!(decisions.stop_hiding(|_| true).is_empty(), "then forgotten");
}

#[test]
fn a_hidden_console_is_left_as_it_is() {
    let mut decisions = Decisions::default();
    assert_eq!(decisions.member(&seen(1, false), true), Action::Nothing);
    assert!(decisions.stop_hiding(|_| true).is_empty());
}

#[test]
fn nothing_is_hidden_while_consoles_are_shown() {
    let mut decisions = Decisions::default();
    assert_eq!(decisions.member(&seen(1, true), false), Action::Nothing);
}

#[test]
fn a_console_shown_again_is_hidden_again() {
    let mut decisions = Decisions::default();
    assert_eq!(decisions.member(&seen(1, true), true), Action::Hide);
    assert_eq!(decisions.member(&seen(1, false), true), Action::Nothing);
    assert_eq!(decisions.member(&seen(1, true), true), Action::Hide);
}

#[test]
fn a_paused_console_is_revealed_once_and_never_hidden_again() {
    let mut decisions = Decisions::default();
    let paused = Seen {
        paused: true,
        ..seen(5, false)
    };
    assert_eq!(decisions.member(&paused, true), Action::Reveal);

    assert_eq!(decisions.member(&seen(5, true), true), Action::Nothing);
    assert_eq!(decisions.member(&paused, true), Action::Nothing);
    assert!(decisions.stop_hiding(|_| true).is_empty());
}

#[test]
fn a_paused_console_already_shown_is_only_left_alone() {
    let mut decisions = Decisions::default();
    assert_eq!(decisions.member(&seen(6, true), true), Action::Hide);
    let paused = Seen {
        paused: true,
        ..seen(6, true)
    };
    assert_eq!(decisions.member(&paused, true), Action::Nothing);
    assert!(
        decisions.stop_hiding(|_| true).is_empty(),
        "a revealed window is not one the watcher hid"
    );
}

#[test]
fn stopping_shows_only_windows_that_still_exist() {
    let mut decisions = Decisions::default();
    for window in [1, 2, 3] {
        decisions.member(&seen(window, true), true);
    }
    assert_eq!(
        decisions.stop_hiding(|window| window != 2),
        vec![(1, false), (3, false)]
    );
}

#[test]
fn a_console_created_hidden_for_a_started_program_is_shown_when_hiding_stops() {
    let mut decisions = Decisions::default();
    assert_eq!(
        decisions.started_hidden(&seen(7, false)),
        Action::Show { minimized: false }
    );
    assert_eq!(decisions.started_hidden(&seen(7, true)), Action::Nothing);
    let paused = Seen {
        paused: true,
        ..seen(8, false)
    };
    decisions.member(&paused, true);
    assert_eq!(decisions.started_hidden(&seen(8, false)), Action::Nothing);
}

#[test]
fn gone_windows_are_forgotten() {
    let mut decisions = Decisions::default();
    decisions.member(&seen(1, true), true);
    decisions.member(&seen(2, true), true);
    decisions.forget_gone(|window| window == 2);
    assert_eq!(decisions.stop_hiding(|_| true), vec![(2, false)]);
}
