use super::*;

#[test]
fn discovery_excludes_prelaunch_pids_without_guessing_between_new_matches() {
    let baseline = BTreeSet::from([10, 20]);
    assert_eq!(new_candidates(vec![10, 20, 30], &baseline), vec![30]);
    assert_eq!(new_candidates(vec![10, 30, 40], &baseline), vec![30, 40]);
}

#[test]
fn bounded_discovery_is_cancellable_without_waiting_for_timeout() {
    let sink = Arc::new(|_event: WatchEvent| {});
    let mut watch = EscapedProcessWatch::start(
        GameId(7),
        "gametoaster-this-process-does-not-exist.exe".into(),
        None,
        BTreeSet::new(),
        sink,
    )
    .unwrap();
    let started = Instant::now();
    watch.stop();
    assert!(started.elapsed() < Duration::from_secs(1));
}
