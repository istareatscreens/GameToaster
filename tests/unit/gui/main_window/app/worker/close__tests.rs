use super::*;

#[test]
fn only_a_kill_that_ended_a_direct_launch_observed_the_exit() {
    assert!(exit_observed(CloseMode::KillExe, SessionClosed::Removed));

    assert!(!exit_observed(
        CloseMode::KillExe,
        SessionClosed::PluginCancelled
    ));

    assert!(!exit_observed(CloseMode::KillExe, SessionClosed::None));

    assert!(!exit_observed(CloseMode::Script, SessionClosed::Removed));
    assert!(!exit_observed(
        CloseMode::Script,
        SessionClosed::PluginCancelled
    ));
    assert!(!exit_observed(CloseMode::Script, SessionClosed::None));
}
