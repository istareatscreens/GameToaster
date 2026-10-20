use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use super::super::fakes::{fake_adapter, FAKE_DESCRIPTOR};
use super::*;
use crate::gui::game_launcher_screen::domain::command::LauncherCommand;

type ManagerFixture = (
    ControllerManager,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
    Arc<Mutex<Option<EventSink>>>,
);

fn manager_with(leaks_a_handle: bool) -> ManagerFixture {
    let connects = Arc::new(AtomicUsize::new(0));
    let disconnects = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(Mutex::new(None));
    let mut registry = AdapterRegistry::default();
    registry
        .register(fake_adapter(
            connects.clone(),
            disconnects.clone(),
            sink.clone(),
            leaks_a_handle,
        ))
        .unwrap();
    (
        ControllerManager::new(registry),
        connects,
        disconnects,
        sink,
    )
}

fn manager() -> ManagerFixture {
    manager_with(false)
}

fn config() -> ControllerConfig {
    FAKE_DESCRIPTOR.default_config()
}

#[test]
fn reuses_one_session_for_the_same_configuration() {
    let (mut manager, connects, disconnects, _) = manager();
    let config = config();
    manager.connect(&config).unwrap();
    manager.connect(&config).unwrap();
    assert_eq!(connects.load(Ordering::SeqCst), 1);
    assert_eq!(disconnects.load(Ordering::SeqCst), 0);
    assert!(manager.is_active());
    assert_eq!(
        manager.disconnect(RELEASE_TIMEOUT).unwrap(),
        ReleaseProof::default()
    );
    assert_eq!(disconnects.load(Ordering::SeqCst), 1);
}

#[test]
fn the_release_proof_is_read_from_the_session_audit() {
    let (mut manager, _, disconnects, _) = manager_with(true);
    manager.connect(&config()).unwrap();
    assert_eq!(manager.resource_counts(), (1, 0));
    let error = manager.disconnect(RELEASE_TIMEOUT).unwrap_err();
    assert!(error.contains("1 handle(s), 0 reader(s) remain"), "{error}");

    assert_eq!(disconnects.load(Ordering::SeqCst), 1);
    assert!(manager.is_active());
    assert_eq!(manager.resource_counts(), (1, 0));
}

#[test]
fn reconnect_baseline_suppresses_a_held_button() {
    let (mut manager, _, _, sink) = manager();
    manager.connect(&config()).unwrap();
    let held = ButtonWord::from_bits(Button::Ba4.mask());
    {
        let guard = sink.lock().unwrap();
        let sink = guard.as_ref().unwrap();
        sink.try_send(SessionEvent::Connected).unwrap();
        sink.try_send(SessionEvent::Snapshot(held)).unwrap();
    }
    assert!(!manager.poll().iter().any(|event| matches!(
        event,
        ManagerEvent::Edge {
            button: Button::Ba4,
            pressed: true
        }
    )));
    {
        let guard = sink.lock().unwrap();
        let sink = guard.as_ref().unwrap();
        sink.try_send(SessionEvent::Snapshot(ButtonWord::EMPTY))
            .unwrap();
        sink.try_send(SessionEvent::Snapshot(held)).unwrap();
    }
    assert!(manager.poll().iter().any(|event| matches!(
        event,
        ManagerEvent::Edge {
            button: Button::Ba4,
            pressed: true
        }
    )));
}

#[test]
fn reconfiguration_releases_the_old_session_before_connecting_once() {
    let (mut manager, connects, disconnects, _) = manager();
    let first = config();
    let mut second = first.clone();
    second.options.insert("pid".into(), "0022".into());
    manager.connect(&first).unwrap();
    manager.connect(&second).unwrap();
    assert_eq!(connects.load(Ordering::SeqCst), 2);
    assert_eq!(disconnects.load(Ordering::SeqCst), 1);
    manager.disconnect(RELEASE_TIMEOUT).unwrap();
    assert_eq!(disconnects.load(Ordering::SeqCst), 2);
    assert!(!manager.is_active());
}

#[test]
fn invalid_configuration_is_rejected_before_adapter_open() {
    let (mut manager, connects, disconnects, _) = manager();
    let mut config = config();
    config.options.insert("pid".into(), "0000".into());
    assert!(manager.connect(&config).is_err());
    assert_eq!(connects.load(Ordering::SeqCst), 0);
    assert_eq!(disconnects.load(Ordering::SeqCst), 0);
}

#[test]
fn command_for_follows_the_active_drivers_bindings() {
    let (mut manager, _, _, _) = manager();
    assert_eq!(
        manager.command_for(Button::Ba4),
        None,
        "no session, no command"
    );
    manager.connect(&config()).unwrap();
    assert_eq!(
        manager.command_for(Button::Ba4),
        Some(LauncherCommand::Launch)
    );
    assert_eq!(
        manager.command_for(Button::Ba1),
        None,
        "an unbound button does nothing"
    );
    manager.disconnect(RELEASE_TIMEOUT).unwrap();
    assert_eq!(manager.command_for(Button::Ba4), None);
}
