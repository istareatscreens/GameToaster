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

const GONE: u32 = u32::MAX - 3;

fn expire(hider: &mut Option<ConsoleHider>) {
    hider.as_mut().unwrap().deadline = Instant::now() - Duration::from_secs(1);
}

#[test]
fn nothing_is_followed_while_consoles_are_shown_or_paused() {
    for settings in [SHOWN, PAUSE] {
        let mut hider = None;
        follow_root(&mut hider, settings, GONE, 1);
        follow_watch_target(&mut hider, settings, "game.exe");
        assert!(hider.is_none(), "{settings:?}");
        assert!(!tick(&mut hider, settings));
    }
}

#[test]
fn a_root_or_a_watch_target_starts_the_hider_while_consoles_are_hidden() {
    let mut hider = None;
    follow_root(&mut hider, HIDDEN, GONE, 1);
    assert!(hider.is_some());
    let mut hider = None;
    follow_watch_target(&mut hider, HIDDEN, "game.exe");
    assert_eq!(hider.unwrap().watch_targets, ["game.exe"]);
}

#[test]
fn watch_targets_are_kept_once_and_blank_ones_are_ignored() {
    let mut hider = None;
    for name in ["game.exe", "  ", "GAME.EXE", "other.exe "] {
        follow_watch_target(&mut hider, HIDDEN, name);
    }
    assert_eq!(hider.unwrap().watch_targets, ["game.exe", "other.exe"]);
}

#[test]
fn showing_consoles_stops_the_hider() {
    let mut hider = None;
    follow_root(&mut hider, HIDDEN, GONE, 1);
    assert!(tick(&mut hider, HIDDEN));
    assert!(!tick(&mut hider, SHOWN));
    assert!(hider.is_none());
}

#[test]
fn a_root_reported_later_extends_the_watch_window() {
    let mut hider = None;
    follow_root(&mut hider, HIDDEN, GONE, 1);
    expire(&mut hider);
    follow_root(&mut hider, HIDDEN, GONE, 2);
    assert!(
        tick(&mut hider, HIDDEN),
        "the second root reopened the window"
    );
}

#[test]
fn the_hider_stops_when_its_watch_window_ends_and_a_new_root_starts_another() {
    let mut hider = None;
    follow_root(&mut hider, HIDDEN, GONE, 1);
    expire(&mut hider);
    assert!(!tick(&mut hider, HIDDEN));
    assert!(hider.is_none());
    follow_root(&mut hider, HIDDEN, GONE, 3);
    assert!(tick(&mut hider, HIDDEN));
}

#[test]
fn a_pass_runs_at_most_every_interval_however_often_the_frame_ticks() {
    let mut hider = None;
    follow_root(&mut hider, HIDDEN, GONE, 1);
    assert!(tick(&mut hider, HIDDEN));
    let next = hider.as_ref().unwrap().next_pass;
    assert!(next > Instant::now(), "the first pass ran and set the next");
    assert!(tick(&mut hider, HIDDEN));
    assert_eq!(
        hider.as_ref().unwrap().next_pass,
        next,
        "a tick between passes does nothing"
    );
    hider.as_mut().unwrap().next_pass = Instant::now() - Duration::from_millis(1);
    assert!(tick(&mut hider, HIDDEN));
    assert!(
        hider.as_ref().unwrap().next_pass > next,
        "the next pass ran"
    );
}
