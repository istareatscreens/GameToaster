use super::*;

#[test]
fn only_enabled_states_start_the_app_at_sign_in() {
    assert!(starts_at_sign_in(StartupTaskState::Enabled));
    assert!(starts_at_sign_in(StartupTaskState::EnabledByPolicy));
    assert!(!starts_at_sign_in(StartupTaskState::Disabled));
    assert!(!starts_at_sign_in(StartupTaskState::DisabledByUser));
    assert!(!starts_at_sign_in(StartupTaskState::DisabledByPolicy));
}

#[test]
fn an_enable_request_reports_why_the_task_stays_off() {
    let _locale = crate::gui::common::harness::locale("en");
    assert_eq!(enable_outcome(StartupTaskState::Enabled), Ok(()));
    assert_eq!(enable_outcome(StartupTaskState::EnabledByPolicy), Ok(()));
    let by_user = enable_outcome(StartupTaskState::DisabledByUser).unwrap_err();
    assert!(
        by_user.contains("Task Manager") && by_user.contains("Settings > Apps > Startup"),
        "{by_user}"
    );
    assert!(enable_outcome(StartupTaskState::DisabledByPolicy)
        .unwrap_err()
        .contains("policy"));
    assert!(enable_outcome(StartupTaskState::Disabled).is_err());
}

#[test]
fn the_state_is_queried_at_most_once_per_refresh_interval() {
    let mut memo = StateMemo { last: None };
    let start = Instant::now();
    let mut queries = 0;
    let mut query = |value: bool| {
        queries += 1;
        value
    };
    assert!(memo.get(start, || query(true)));
    assert!(memo.get(start + STATE_REFRESH / 2, || query(false)));
    assert!(!memo.get(start + STATE_REFRESH, || query(false)));
    memo.invalidate();
    assert!(memo.get(start + STATE_REFRESH, || query(true)));
    assert_eq!(queries, 3);
}

#[test]
fn the_task_id_matches_the_package_manifest() {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging/msix/AppxManifest.xml"),
    )
    .unwrap();
    assert!(
        manifest.contains(&format!("TaskId=\"{TASK_ID}\"")),
        "the manifest must declare the startup task {TASK_ID}"
    );
}
