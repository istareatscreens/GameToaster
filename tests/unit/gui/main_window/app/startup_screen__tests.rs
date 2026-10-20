use super::*;

fn tray(open_on_os_startup: bool, monitor_return_configured: bool) -> StartupScreenInputs {
    StartupScreenInputs {
        enabled: true,
        started_in_tray: true,
        open_on_os_startup,
        launch_on_start: true,
        controller_only_start: true,
        monitor_return_configured,
    }
}

fn user_open(launch_on_start: bool, controller_only_start: bool) -> StartupScreenInputs {
    StartupScreenInputs {
        enabled: true,
        started_in_tray: false,
        open_on_os_startup: true,
        launch_on_start,
        controller_only_start,
        monitor_return_configured: true,
    }
}

fn must_not_be_asked() -> bool {
    panic!("display availability must not be consulted for this decision")
}

#[test]
fn a_disabled_feature_never_opens_and_never_enumerates_displays() {
    for started_in_tray in [true, false] {
        let inputs = StartupScreenInputs {
            enabled: false,
            started_in_tray,
            open_on_os_startup: true,
            launch_on_start: true,
            controller_only_start: true,
            monitor_return_configured: true,
        };
        assert_eq!(
            decide(inputs, must_not_be_asked),
            StartupScreenAction::Nothing
        );
    }
}

#[test]
fn os_autostart_stays_silent_unless_open_on_os_startup_is_enabled() {
    assert_eq!(
        decide(tray(false, true), must_not_be_asked),
        StartupScreenAction::Nothing
    );
}

#[test]
fn os_autostart_with_the_display_present_opens_in_the_tray() {
    assert_eq!(
        decide(tray(true, true), || true),
        StartupScreenAction::OpenInTray
    );
    assert_eq!(
        decide(tray(true, false), || true),
        StartupScreenAction::OpenInTray
    );
}

#[test]
fn os_autostart_with_the_display_absent_arms_the_return_when_one_is_configured() {
    assert_eq!(
        decide(tray(true, true), || false),
        StartupScreenAction::ArmMonitorReturn
    );
}

#[test]
fn os_autostart_with_the_display_absent_reveals_the_error_when_nothing_could_reopen() {
    assert_eq!(
        decide(tray(true, false), || false),
        StartupScreenAction::RevealMonitorMissing
    );
}

#[test]
fn a_user_open_honours_launch_on_start_and_controller_only() {
    assert_eq!(
        decide(user_open(true, true), must_not_be_asked),
        StartupScreenAction::OpenForUser {
            controller_only: true
        }
    );
    assert_eq!(
        decide(user_open(true, false), must_not_be_asked),
        StartupScreenAction::OpenForUser {
            controller_only: false
        }
    );
    assert_eq!(
        decide(user_open(false, true), must_not_be_asked),
        StartupScreenAction::Nothing
    );
}

#[test]
fn launch_on_start_never_fires_on_an_os_boot() {
    let mut inputs = tray(false, true);
    inputs.launch_on_start = true;
    assert_eq!(
        decide(inputs, must_not_be_asked),
        StartupScreenAction::Nothing
    );
}
