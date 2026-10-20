use super::*;
use std::fs;
use std::time::{Duration, Instant};

#[test]
fn dirty_state_coalesces_notifications_overflow_and_failure() {
    let state = HintState::default();
    state.mark_notification();
    state.mark_notification();
    state.mark_overflow();
    state.mark_failure();
    assert!(state.dirty.swap(false, Ordering::AcqRel));
    assert!(!state.dirty.swap(false, Ordering::AcqRel));
    assert_eq!(state.notifications.load(Ordering::Relaxed), 2);
    assert_eq!(state.overflows.load(Ordering::Relaxed), 1);
    assert_eq!(state.failures.load(Ordering::Relaxed), 1);
}

#[test]
fn real_directory_change_becomes_one_dirty_hint() {
    let root = tempfile::tempdir().unwrap();
    let watcher = CatalogWatcher::start(root.path()).unwrap();
    fs::write(root.path().join("manifest.toml"), b"changed").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !watcher.take_dirty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(watcher.stats().notifications > 0);
    assert!(!watcher.take_dirty());
}
