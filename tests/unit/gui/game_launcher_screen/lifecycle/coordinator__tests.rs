use super::*;

#[test]
fn spawn_is_impossible_before_zero_resource_release() {
    let mut lifecycle = LifecycleCoordinator::new(true);

    assert_eq!(
        lifecycle
            .request_launch(GameId(4), LaunchSource::Controller, true)
            .unwrap(),
        Vec::new()
    );
    assert_eq!(lifecycle.state(), State::Preparing);

    assert_eq!(
        lifecycle.begin_handoff().unwrap(),
        vec![Action::HideControllerViewport, Action::DisconnectController]
    );
    assert!(lifecycle.controller_released(1, 0).is_err());
    assert_eq!(lifecycle.state(), State::Idle);

    lifecycle
        .request_launch(GameId(4), LaunchSource::Controller, true)
        .unwrap();

    lifecycle.begin_handoff().unwrap();
    assert_eq!(
        lifecycle.controller_released(0, 0).unwrap(),
        Action::Spawn(GameId(4))
    );
    lifecycle.launch_succeeded().unwrap();
    assert_eq!(
        lifecycle.game_exited(),
        vec![Action::ShowControllerViewport, Action::ConnectController]
    );
    lifecycle.restore_complete();
    assert_eq!(lifecycle.state(), State::Idle);
}

#[test]
fn kiosk_always_reopens_even_when_controller_was_closed() {
    let mut lifecycle = LifecycleCoordinator::new(true);
    lifecycle.set_always_reopen(true);
    lifecycle
        .request_launch(GameId(5), LaunchSource::Tray, false)
        .unwrap();
    lifecycle.begin_handoff().unwrap();
    lifecycle.controller_released(0, 0).unwrap();
    lifecycle.launch_succeeded().unwrap();
    assert_eq!(
        lifecycle.game_exited(),
        vec![Action::ShowControllerViewport, Action::ConnectController]
    );
    lifecycle.restore_complete();
    assert_eq!(lifecycle.state(), State::Idle);
}

#[test]
fn desktop_does_not_reopen_a_closed_controller_after_exit() {
    let mut lifecycle = LifecycleCoordinator::new(true);
    lifecycle
        .request_launch(GameId(6), LaunchSource::MainGui, false)
        .unwrap();
    lifecycle.begin_handoff().unwrap();
    lifecycle.controller_released(0, 0).unwrap();
    lifecycle.launch_succeeded().unwrap();
    assert!(lifecycle.game_exited().is_empty());
    assert_eq!(lifecycle.state(), State::Idle);
}

#[test]
fn shutdown_suppresses_late_restore() {
    let mut lifecycle = LifecycleCoordinator::new(true);
    lifecycle
        .request_launch(GameId(1), LaunchSource::MainGui, true)
        .unwrap();
    lifecycle.begin_handoff().unwrap();
    lifecycle.controller_released(0, 0).unwrap();
    lifecycle.launch_succeeded().unwrap();
    lifecycle.shutdown();
    assert!(lifecycle.game_exited().is_empty());
    assert_eq!(lifecycle.state(), State::ShuttingDown);
}

#[test]
fn every_launch_source_uses_the_same_serialized_transition_path() {
    for source in [
        LaunchSource::MainGui,
        LaunchSource::Controller,
        LaunchSource::Tray,
        LaunchSource::Hotkey,
        LaunchSource::Startup,
    ] {
        let mut lifecycle = LifecycleCoordinator::new(false);
        assert_eq!(
            lifecycle.request_launch(GameId(8), source, false).unwrap(),
            Vec::new()
        );
        assert!(lifecycle
            .request_launch(GameId(9), source, false)
            .unwrap_err()
            .contains("busy"));
        assert_eq!(
            lifecycle.begin_handoff().unwrap(),
            vec![Action::HideControllerViewport, Action::DisconnectController]
        );
        assert_eq!(
            lifecycle.controller_released(0, 0).unwrap(),
            Action::Spawn(GameId(8))
        );
        lifecycle.launch_succeeded().unwrap();
        assert!(lifecycle.game_exited().is_empty());
        assert_eq!(lifecycle.state(), State::Idle);
    }
}

#[test]
fn launch_failure_restores_only_a_previously_open_controller() {
    let mut lifecycle = LifecycleCoordinator::new(true);
    lifecycle
        .request_launch(GameId(2), LaunchSource::Controller, true)
        .unwrap();
    lifecycle.begin_handoff().unwrap();
    lifecycle.controller_released(0, 0).unwrap();
    assert_eq!(
        lifecycle.launch_failed(),
        vec![Action::ShowControllerViewport, Action::ConnectController]
    );

    lifecycle
        .request_launch(GameId(2), LaunchSource::MainGui, false)
        .unwrap();

    lifecycle.begin_handoff().unwrap();
    lifecycle.controller_released(0, 0).unwrap();
    assert!(lifecycle.launch_failed().is_empty());
}

#[test]
fn a_handoff_can_only_be_asked_for_while_preparing() {
    let mut lifecycle = LifecycleCoordinator::new(true);

    assert!(lifecycle.begin_handoff().is_err());
    lifecycle
        .request_launch(GameId(7), LaunchSource::Controller, true)
        .unwrap();
    lifecycle.begin_handoff().unwrap();

    assert!(lifecycle.begin_handoff().is_err());
    lifecycle.controller_released(0, 0).unwrap();
    assert!(lifecycle.begin_handoff().is_err());
}

#[test]
fn a_launch_cancelled_while_preparing_restores_the_screen_it_was_started_from() {
    let mut lifecycle = LifecycleCoordinator::new(true);
    lifecycle
        .request_launch(GameId(8), LaunchSource::Controller, true)
        .unwrap();
    assert_eq!(lifecycle.state(), State::Preparing);

    assert_eq!(
        lifecycle.launch_failed(),
        vec![Action::ShowControllerViewport, Action::ConnectController]
    );
    assert_eq!(lifecycle.state(), State::Idle);
}

#[test]
fn a_launch_is_in_progress_from_the_request_so_a_second_one_is_refused() {
    let mut lifecycle = LifecycleCoordinator::new(true);
    lifecycle
        .request_launch(GameId(9), LaunchSource::MainGui, false)
        .unwrap();
    assert!(lifecycle
        .request_launch(GameId(10), LaunchSource::MainGui, false)
        .is_err());
}
