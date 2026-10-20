use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use super::super::buttons::Button;
use super::super::descriptor::ButtonBinding;
use super::super::fakes::{fake_adapter, FAKE_DESCRIPTOR};
use super::*;
use crate::gui::game_launcher_screen::domain::command::LauncherCommand;

#[test]
fn fake_driver_registers_without_manager_changes_and_duplicates_are_rejected() {
    let connects = Arc::new(AtomicUsize::new(0));
    let disconnects = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(Mutex::new(None));
    let mut registry = AdapterRegistry::default();
    registry
        .register(fake_adapter(
            connects.clone(),
            disconnects.clone(),
            sink.clone(),
            false,
        ))
        .unwrap();
    let duplicate = registry.register(fake_adapter(connects, disconnects, sink, false));
    assert!(duplicate.unwrap_err().contains("already registered"));
    assert!(registry.default_config("missing-driver").is_err());
    let discovered = registry.discover().unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].config, FAKE_DESCRIPTOR.default_config());
}

#[test]
fn a_driver_binding_one_button_twice_is_refused() {
    static TWICE: ControllerDriverDescriptor = ControllerDriverDescriptor {
        id: "twice-bound",
        label_key: "controller.type.adx_io4",
        recommended_layout: "washing-machine",
        options: &[],
        lights: None,
        bindings: &[
            ButtonBinding {
                button: Button::Ba4,
                command: LauncherCommand::Launch,
            },
            ButtonBinding {
                button: Button::Ba4,
                command: LauncherCommand::Next,
            },
        ],
    };
    struct TwiceBound;
    impl ControllerAdapter for TwiceBound {
        fn descriptor(&self) -> &'static ControllerDriverDescriptor {
            &TWICE
        }

        fn connect(
            &self,
            _config: &ControllerConfig,
            _events: super::super::events::EventSink,
            _audit: Arc<super::super::audit::ResourceAudit>,
        ) -> Result<Box<dyn super::super::adapter::ControllerSession>, String> {
            Err("never connected".into())
        }
    }
    let error = AdapterRegistry::default()
        .register(Arc::new(TwiceBound))
        .unwrap_err();
    assert!(
        error.contains("'twice-bound' binds button BA4 more than once"),
        "{error}"
    );
}

#[test]
fn the_built_in_registry_is_the_three_ring_devices() {
    let registry = AdapterRegistry::with_default_adapters();
    let ids: Vec<&str> = registry
        .descriptors()
        .iter()
        .map(|driver| driver.id)
        .collect();
    assert_eq!(ids, vec!["adx-hid", "adx-io4", "jvs"]);

    let mut registry = registry;
    registry
        .register(fake_adapter(
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(Mutex::new(None)),
            false,
        ))
        .unwrap();
    assert!(registry.default_config(FAKE_DESCRIPTOR.id).is_ok());
}
