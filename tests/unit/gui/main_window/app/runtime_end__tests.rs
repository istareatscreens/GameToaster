use super::*;

fn failed() -> RuntimeEnd {
    RuntimeEnd::Failed {
        title: "Plugin runtime stopped".into(),
        message: "Plugin process exited with code 3 (LaunchFailure)".into(),
    }
}

#[test]
fn the_end_of_a_confirmed_game_session_is_never_a_dialog() {
    assert_eq!(dialog_for(&RuntimeEnd::Clean, true), None);
    assert_eq!(dialog_for(&RuntimeEnd::Cancelled, true), None);
    assert_eq!(dialog_for(&failed(), true), None);
}

#[test]
fn a_launcher_cancel_is_never_a_dialog_even_before_confirmation() {
    assert_eq!(dialog_for(&RuntimeEnd::Cancelled, false), None);
    assert_eq!(dialog_for(&RuntimeEnd::Clean, false), None);
}

#[test]
fn a_failure_before_the_game_was_confirmed_is_reported() {
    assert_eq!(
        dialog_for(&failed(), false),
        Some((
            "Plugin runtime stopped".to_string(),
            "Plugin process exited with code 3 (LaunchFailure)".to_string()
        ))
    );
}
