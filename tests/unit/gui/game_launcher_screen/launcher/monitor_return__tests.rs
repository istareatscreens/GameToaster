use std::time::Duration;

use super::super::tests::{absent_display_fixture, monitor_return_fixture, test_launcher};
use super::*;
use crate::game_launcher_screen::ControllerLaunchSource;
use crate::launcher_gui::model::GameId;

#[test]
fn monitor_loss_releases_screen_and_return_reopens_once_when_idle() {
    let Some((config, screen)) = monitor_return_fixture() else {
        return;
    };
    let mut launcher = test_launcher(&config, "monitor-return");
    launcher.open(&screen).unwrap();
    let target = MonitorTarget::from_config(&screen).unwrap();

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Unavailable(target.clone()), &screen);
    assert!(!launcher.is_open());
    assert_eq!(
        launcher.monitor_return,
        MonitorReturnState::Waiting(target.clone())
    );
    assert_eq!(launcher.resource_counts(), (0, 0));

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target.clone()), &screen);
    assert_eq!(
        launcher.monitor_return,
        MonitorReturnState::Pending(target.clone())
    );
    launcher.complete_monitor_return(&screen);
    assert!(launcher.is_open());
    assert_eq!(launcher.monitor_return, MonitorReturnState::Inactive);
    launcher.close().unwrap();

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target.clone()), &screen);
    assert_eq!(
        launcher.monitor_return,
        MonitorReturnState::Pending(target.clone())
    );
    launcher.complete_monitor_return(&screen);
    assert!(launcher.is_open());
    launcher.close().unwrap();

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Unavailable(target.clone()), &screen);
    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target), &screen);
    launcher.complete_monitor_return(&screen);
    assert!(launcher.is_open());
    launcher.close().unwrap();
}

#[test]
fn a_cold_start_return_reopens_without_a_self_observed_absence() {
    let Some((config, screen)) = monitor_return_fixture() else {
        return;
    };

    let mut launcher = test_launcher(&config, "monitor-return-cold-start");
    let target = MonitorTarget::from_config(&screen).unwrap();
    assert!(!launcher.monitor_return_is_armed());
    assert!(!launcher.is_open());

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target.clone()), &screen);
    assert_eq!(launcher.monitor_return, MonitorReturnState::Pending(target));
    launcher.complete_monitor_return(&screen);
    assert!(launcher.is_open());
    assert_eq!(launcher.monitor_return, MonitorReturnState::Inactive);
    launcher.close().unwrap();
}

#[test]
fn a_return_for_a_target_that_no_longer_matches_is_discarded() {
    let Some((config, screen)) = monitor_return_fixture() else {
        return;
    };
    let mut launcher = test_launcher(&config, "monitor-return-stale");
    let mut other_config = screen.clone();
    other_config.monitor_adapter = Some(other_config.monitor_adapter.unwrap().wrapping_add(1));
    other_config.calibration = None;
    let other = MonitorTarget::from_config(&other_config).unwrap();
    assert_ne!(other, MonitorTarget::from_config(&screen).unwrap());

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(other), &screen);
    assert_eq!(launcher.monitor_return, MonitorReturnState::Inactive);
    launcher.complete_monitor_return(&screen);
    assert!(!launcher.is_open());
}

#[test]
fn os_startup_arms_the_return_only_when_one_is_configured() {
    let Some((config, screen)) = monitor_return_fixture() else {
        return;
    };
    let mut launcher = test_launcher(&config, "monitor-return-os-startup");
    assert!(ControllerLauncher::monitor_return_configured(&screen));
    assert!(launcher.arm_monitor_return(&screen));
    assert!(launcher.monitor_return_is_armed());
    assert!(!launcher.is_open());

    let target = MonitorTarget::from_config(&screen).unwrap();
    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target), &screen);
    launcher.complete_monitor_return(&screen);
    assert!(launcher.is_open());
    launcher.close().unwrap();

    let mut disabled = screen.clone();
    disabled.reopen_on_monitor_return = false;
    let mut launcher = test_launcher(&config, "monitor-return-os-startup-disabled");
    assert!(!ControllerLauncher::monitor_return_configured(&disabled));
    assert!(!launcher.arm_monitor_return(&disabled));
    assert!(!launcher.monitor_return_is_armed());
}

#[test]
fn monitor_return_waits_for_game_lifecycle_idle_and_ignores_stale_targets() {
    let Some((config, screen)) = monitor_return_fixture() else {
        return;
    };
    let mut launcher = test_launcher(&config, "monitor-return-lifecycle");
    launcher.open(&screen).unwrap();
    let target = MonitorTarget::from_config(&screen).unwrap();
    assert!(launcher
        .prepare_game_handoff(GameId(44), ControllerLaunchSource::Controller, false, false,)
        .unwrap());
    launcher.launch_succeeded().unwrap();

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Unavailable(target.clone()), &screen);
    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target.clone()), &screen);
    launcher.complete_monitor_return(&screen);
    assert!(!launcher.is_open());
    assert_eq!(
        launcher.monitor_return,
        MonitorReturnState::Pending(target.clone())
    );

    assert!(!launcher.game_exited());
    launcher.complete_monitor_return(&screen);
    assert!(launcher.is_open());
    assert_eq!(launcher.monitor_return, MonitorReturnState::Inactive);
    launcher.close().unwrap();

    let mut changed = screen.clone();
    changed.monitor_adapter = Some(changed.monitor_adapter.unwrap().wrapping_add(1));
    changed.calibration = None;
    launcher.handle_monitor_watch_event(MonitorWatchEvent::Unavailable(target), &changed);
    assert_eq!(launcher.monitor_return, MonitorReturnState::Inactive);
    assert!(!launcher.is_open());
}

#[test]
fn a_failed_return_open_is_retried_with_backoff_and_eventually_gives_up() {
    let Some((config, screen)) = absent_display_fixture() else {
        return;
    };
    let mut launcher = test_launcher(&config, "return-retry");
    let target = MonitorTarget::from_config(&screen).unwrap();
    let t0 = Instant::now();

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target.clone()), &screen);
    launcher.complete_monitor_return_at(&screen, t0);

    assert!(launcher.monitor_return_is_armed());
    assert!(!launcher.is_open());
    assert_eq!(launcher.return_retry.map(|retry| retry.attempts), Some(1));
    assert!(matches!(
        launcher.status,
        ControllerStatus::DisplayChanged(_)
    ));

    launcher.complete_monitor_return_at(&screen, t0 + Duration::from_millis(500));
    assert_eq!(launcher.return_retry.map(|retry| retry.attempts), Some(1));
    launcher.complete_monitor_return_at(&screen, t0 + Duration::from_secs(1));
    assert_eq!(launcher.return_retry.map(|retry| retry.attempts), Some(2));

    let mut now = t0 + Duration::from_secs(1);
    for attempt in 2..RETURN_RETRY_ATTEMPTS {
        let retry = launcher.return_retry.expect("still retrying");
        assert_eq!(retry.attempts, attempt);
        assert!(retry.next_attempt - now <= RETURN_RETRY_MAX);
        now = retry.next_attempt;
        launcher.complete_monitor_return_at(&screen, now);
    }
    assert!(!launcher.monitor_return_is_armed());
    assert!(launcher.return_retry.is_none());

    launcher.handle_monitor_watch_event(MonitorWatchEvent::Returned(target), &screen);
    assert!(launcher.monitor_return_is_armed());
    assert!(launcher.return_retry.is_none());
}

#[test]
fn a_waiting_return_is_completed_when_the_display_is_observed_present() {
    let Some((config, screen)) = monitor_return_fixture() else {
        return;
    };
    let mut launcher = test_launcher(&config, "return-level");
    let target = MonitorTarget::from_config(&screen).unwrap();

    assert!(launcher.arm_monitor_return(&screen));
    launcher
        .monitor_watcher
        .record_presence_for_test(&target, true);
    launcher.poll_monitor_watcher(&screen);

    assert!(launcher.monitor_return_is_armed());
    assert!(!launcher.is_open());
    launcher.complete_monitor_return_at(&screen, Instant::now() + RETURN_RETRY_STEP);
    assert!(
        launcher.is_open(),
        "a present display completes a waiting return (status: {}; return={:?}; retry={:?}; probe_error={:?})",
        launcher.status(),
        launcher.monitor_return,
        launcher.return_retry,
        launcher.monitor_watcher.probe_error(&target)
    );

    launcher.close().unwrap();
    launcher
        .monitor_watcher
        .record_presence_for_test(&target, true);
    launcher.poll_monitor_watcher(&screen);
    assert!(!launcher.is_open());
    assert!(!launcher.monitor_return_is_armed());
}
