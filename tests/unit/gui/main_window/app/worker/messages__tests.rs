use super::*;

#[test]
fn a_launched_result_names_its_root_process() {
    let result = WorkerResult::Launched {
        game_id: GameId(42),
        root_pid: 1234,
        root_created: 5678,
        updated_config: None,
    };
    let WorkerResult::Launched {
        game_id,
        root_pid,
        root_created,
        updated_config,
    } = result
    else {
        panic!("expected launched result");
    };
    assert_eq!(game_id, GameId(42));
    assert_eq!(root_pid, 1234);
    assert_eq!(root_created, 5678);
    assert!(updated_config.is_none());
}

#[test]
fn a_watched_process_wait_is_armed_only_by_a_successful_launch() {
    use std::{fs, path::Path};

    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gui/main_window/app/handlers.rs"),
    )
    .unwrap();
    let arms: Vec<usize> = source
        .match_indices("\n            WorkerResult::")
        .map(|(index, _)| index)
        .collect();
    assert!(arms.len() > 2, "handler match arms not found");

    let arm_containing = |needle: &str| -> (usize, usize) {
        let position = source
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} not found in handlers.rs"));
        let start = *arms
            .iter()
            .filter(|&&arm| arm < position)
            .max()
            .expect("needle must sit inside a result arm");
        let end = arms
            .iter()
            .copied()
            .find(|&arm| arm > position)
            .unwrap_or(source.len());
        (start, end)
    };

    assert_eq!(
        source.matches("PendingHide::new(").count(),
        1,
        "exactly one place may arm the hide-wait"
    );
    let (arm_start, arm_end) = arm_containing("PendingHide::new(");
    assert!(
        source[arm_start..arm_end].contains("WorkerResult::Launched {"),
        "the hide-wait must be armed only by a successful launch"
    );

    let (error_start, error_end) = arm_containing("WorkerResult::Error { title, message }");
    let error_arm = &source[error_start..error_end];
    assert!(error_arm.contains("controller_launch_pending.take()"));
    assert!(error_arm.contains("launch_failed()"));
    assert!(error_arm.contains("open_controller()"));
    assert!(error_arm.contains("Dialog::error(title, message)"));
    assert!(!error_arm.contains("PendingHide"));
}
