use std::time::{Duration, Instant};

use super::*;

#[test]
fn the_watcher_starts_idle_and_stops_when_dropped() {
    let started = Instant::now();
    let watch = ConsoleWatch::start(true).expect("the watcher starts");
    watch.set_hiding(false);
    watch.set_hiding(true);
    watch.end_game(GameId(1));
    drop(watch);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "it stops at once, with nothing followed"
    );
}

#[test]
fn it_is_active_only_while_consoles_are_hidden_and_something_is_followed() {
    let now = Instant::now();
    let mut watcher = Watcher::new(true);
    assert!(!watcher.active());
    assert_eq!(watcher.wait(now), INFINITE, "idle: no timer");

    watcher.apply(
        Command::FollowGame {
            game: GameId(2),
            pid: 0x7ffff,
            created: 1,
        },
        now,
    );
    assert!(watcher.active());
    assert_eq!(watcher.wait(now), 0, "the first pass runs at once");
    assert!(watcher.fast_until > now, "and the passes turn fast");

    watcher.apply(Command::SetHiding(false), now);
    assert!(!watcher.active(), "consoles shown: nothing to do");
    assert_eq!(watcher.wait(now), INFINITE);
    watcher.apply(Command::SetHiding(true), now);
    assert!(watcher.active());

    watcher.apply(Command::EndGame(GameId(2)), now);
    assert!(!watcher.active(), "the session ended");
    watcher.unhook();
}

#[test]
fn a_followed_program_makes_every_stranger_worth_a_new_look() {
    let now = Instant::now();
    let mut watcher = Watcher::new(true);
    watcher.strangers.insert((42, 7));
    watcher.apply(
        Command::FollowTarget {
            game: GameId(3),
            name: "game.exe".into(),
            path: None,
        },
        now,
    );
    assert!(watcher.strangers.is_empty());
}

#[test]
fn a_close_script_keeps_the_watcher_busy_until_its_time_has_passed() {
    let now = Instant::now();
    let mut watcher = Watcher::new(true);
    watcher.apply(
        Command::FollowTransient {
            pid: 0x7fffe,
            created: 1,
        },
        now,
    );
    assert!(watcher.active());
    watcher.sessions.expire(now + sessions::TRANSIENT_WINDOW);
    assert!(!watcher.active());
}

#[test]
fn a_console_titled_by_the_pause_wrapper_counts_as_paused() {
    let console = ConsoleWindow {
        window: 9,
        owner: 1,
        visible: false,
        minimized: false,
        title: PAUSED_TITLE.into(),
    };
    assert!(seen(&console).paused);

    let as_set = ConsoleWindow {
        title: format!("{PAUSED_TITLE} "),
        ..console.clone()
    };
    assert!(seen(&as_set).paused);
    let running = ConsoleWindow {
        title: "C:\\Windows\\system32\\cmd.exe".into(),
        ..console
    };
    assert!(!seen(&running).paused);
}
