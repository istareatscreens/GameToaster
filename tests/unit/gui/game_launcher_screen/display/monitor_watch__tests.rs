use super::*;

fn target(id: &str) -> MonitorTarget {
    MonitorTarget {
        stable_id: Some(id.into()),
        adapter: Some(1),
    }
}

#[test]
fn only_observed_presence_edges_emit_events() {
    let mut tracker = PresenceTracker::default();
    tracker.set_target(Some(target("cabinet")));

    assert_eq!(tracker.observe(Ok(true)), None);
    assert_eq!(tracker.observe(Ok(true)), None);
    assert_eq!(tracker.observe(Ok(false)), Some(PresenceEdge::Unavailable));
    assert_eq!(tracker.observe(Ok(false)), None);
    assert_eq!(tracker.observe(Ok(true)), Some(PresenceEdge::Returned));
}

#[test]
fn probe_errors_do_not_fabricate_or_reset_edges() {
    let mut tracker = PresenceTracker::default();
    tracker.set_target(Some(target("cabinet")));
    assert_eq!(tracker.observe(Ok(true)), None);
    assert_eq!(tracker.observe(Err("temporary CCD error".into())), None);
    assert_eq!(tracker.observe(Ok(false)), Some(PresenceEdge::Unavailable));
}

#[test]
fn changing_target_reseeds_without_false_return() {
    let mut tracker = PresenceTracker::default();
    tracker.set_target(Some(target("first")));
    assert_eq!(tracker.observe(Ok(false)), None);
    tracker.set_target(Some(target("second")));
    assert_eq!(tracker.observe(Ok(true)), None);
}

#[test]
fn disabled_setting_produces_no_watch_target() {
    let mut config = ControllerScreenConfig {
        enabled: true,
        monitor_adapter: Some(1),
        ..Default::default()
    };
    assert_eq!(MonitorTarget::from_config(&config), None);
    config.reopen_on_monitor_return = true;
    assert!(MonitorTarget::from_config(&config).is_some());
}

#[test]
fn watcher_shutdown_is_bounded_by_command_wakeup() {
    let started = std::time::Instant::now();
    drop(MonitorReturnWatcher::new(egui::Context::default()));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn watcher_has_no_periodic_repaint_path() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/gui/game_launcher_screen/display/monitor_watch.rs"),
    )
    .unwrap();
    assert!(!source.contains("request_repaint_after"));
    assert_eq!(source.matches("ctx.request_repaint();").count(), 1);
    assert!(
        source.find("events.send(event).is_ok()").unwrap()
            < source.find("ctx.request_repaint();").unwrap()
    );
}

#[test]
fn presence_snapshot_is_per_target_and_ignores_probe_errors() {
    let shared: SharedPresence = Arc::new(Mutex::new(None));
    let cabinet = target("cabinet");
    assert_eq!(read_presence(&shared, &cabinet), None);
    assert!(record_observation(
        &shared,
        &cabinet,
        &Err("temporary CCD error".into())
    ));
    assert_eq!(read_presence(&shared, &cabinet), None);
    assert_eq!(
        read_probe_error(&shared, &cabinet).as_deref(),
        Some("temporary CCD error")
    );

    assert!(record_observation(&shared, &cabinet, &Ok(true)));
    assert!(!record_observation(&shared, &cabinet, &Ok(true)));
    assert_eq!(read_presence(&shared, &cabinet), Some(true));
    assert_eq!(read_probe_error(&shared, &cabinet), None);
    assert_eq!(read_presence(&shared, &target("other")), None);
    assert!(record_observation(&shared, &cabinet, &Ok(false)));
    assert_eq!(read_presence(&shared, &cabinet), Some(false));

    let other = target("other");
    let mut tracker = PresenceTracker::default();
    assert!(record_observation(&shared, &other, &Ok(true)));
    assert!(!apply_command(
        &mut tracker,
        Command::SetTarget(Some(other.clone()))
    ));
    assert_eq!(read_presence(&shared, &other), Some(true));
    assert_eq!(read_presence(&shared, &cabinet), None);
    assert!(apply_command(&mut tracker, Command::Stop));
}
