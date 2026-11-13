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
fn the_built_in_registry_is_every_ring_device() {
    let registry = AdapterRegistry::with_default_adapters();
    let ids: Vec<&str> = registry
        .descriptors()
        .iter()
        .map(|driver| driver.id)
        .collect();
    assert_eq!(
        ids,
        vec!["adx-hid", "adx-io4", "dao", "jvs", "ndx-hid", "ndx-io4", "npro"]
    );

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

fn found_by_the_ring_drivers(present: &[(u16, u16, usize)]) -> Vec<DiscoveredController> {
    use super::super::super::devices::{adx_hid, adx_io4, ndx_hid, ndx_io4};
    let mut found = adx_hid::adapter().match_present(present);
    found.extend(adx_io4::adapter().match_present(present));
    found.extend(ndx_hid::adapter().match_present(present));
    found.extend(ndx_io4::adapter().match_present(present));
    found
}

#[test]
fn a_ring_two_drivers_can_read_is_offered_once() {
    let found = found_by_the_ring_drivers(&[(0x2E3C, 0x5750, 33)]);
    assert_eq!(found.len(), 2);
    let offered = offered_once(found);
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].config.driver_id, "adx-hid");

    let offered = offered_once(found_by_the_ring_drivers(&[(0x0CA3, 0x0021, 64)]));
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].config.driver_id, "adx-io4");
}

#[test]
fn different_devices_are_all_offered() {
    let offered = offered_once(found_by_the_ring_drivers(&[
        (0x0CA3, 0x0021, 64),
        (0x2E3C, 0x5750, 33),
    ]));
    let drivers: Vec<&str> = offered
        .iter()
        .map(|controller| controller.config.driver_id.as_str())
        .collect();
    assert_eq!(drivers, vec!["adx-hid", "adx-io4"]);

    let offered = offered_once(found_by_the_ring_drivers(&[
        (0x2E3C, 0x5750, 33),
        (0x2E4C, 0x5750, 33),
    ]));
    assert_eq!(offered.len(), 2);
    assert_eq!(offered[0].config.options.get("vid").unwrap(), "2E3C");
    assert_eq!(offered[1].config.options.get("vid").unwrap(), "2E4C");

    assert!(offered_once(Vec::new()).is_empty());
}
