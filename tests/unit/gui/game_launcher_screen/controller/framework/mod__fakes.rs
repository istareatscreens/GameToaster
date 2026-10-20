use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::adapter::{ControllerAdapter, ControllerSession};
use super::audit::ResourceAudit;
use super::buttons::Button;
use super::descriptor::{
    ButtonBinding, ControllerDriverDescriptor, ControllerOptionDescriptor, ControllerOptionKind,
    DiscoveredController,
};
use super::events::EventSink;
use crate::game_launcher_screen::ControllerConfig;
use crate::gui::game_launcher_screen::domain::command::LauncherCommand;

const FAKE_OPTIONS: &[ControllerOptionDescriptor] = &[
    ControllerOptionDescriptor {
        key: "vid",
        label_key: "controller.type.vid",
        default: "0CA3",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
    ControllerOptionDescriptor {
        key: "pid",
        label_key: "controller.type.pid",
        default: "0021",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
    ControllerOptionDescriptor {
        key: "enabled",
        label_key: "controller.type.select",
        default: "true",
        kind: ControllerOptionKind::Boolean,
    },
];
pub(super) static FAKE_DESCRIPTOR: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "fake-controller",
    label_key: "controller.type.adx_io4",
    recommended_layout: "washing-machine",
    options: FAKE_OPTIONS,
    lights: None,
    bindings: &[ButtonBinding {
        button: Button::Ba4,
        command: LauncherCommand::Launch,
    }],
};

struct FakeAdapter {
    connects: Arc<AtomicUsize>,
    disconnects: Arc<AtomicUsize>,
    sink: Arc<Mutex<Option<EventSink>>>,

    leaks_a_handle: bool,
}

impl ControllerAdapter for FakeAdapter {
    fn descriptor(&self) -> &'static ControllerDriverDescriptor {
        &FAKE_DESCRIPTOR
    }

    fn discover(&self) -> Result<Vec<DiscoveredController>, String> {
        Ok(vec![DiscoveredController {
            config: FAKE_DESCRIPTOR.default_config(),
            label_key: FAKE_DESCRIPTOR.label_key,
        }])
    }

    fn connect(
        &self,
        _config: &ControllerConfig,
        events: EventSink,
        audit: Arc<ResourceAudit>,
    ) -> Result<Box<dyn ControllerSession>, String> {
        self.connects.fetch_add(1, Ordering::SeqCst);
        if self.leaks_a_handle {
            audit.handle_opened();
        }
        *self.sink.lock().unwrap() = Some(events);
        Ok(Box::new(FakeSession {
            disconnects: self.disconnects.clone(),
        }))
    }
}

struct FakeSession {
    disconnects: Arc<AtomicUsize>,
}

impl ControllerSession for FakeSession {
    fn disconnect(&mut self, _timeout: Duration) -> Result<(), String> {
        self.disconnects.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

pub(super) fn fake_adapter(
    connects: Arc<AtomicUsize>,
    disconnects: Arc<AtomicUsize>,
    sink: Arc<Mutex<Option<EventSink>>>,
    leaks_a_handle: bool,
) -> Arc<dyn ControllerAdapter> {
    Arc::new(FakeAdapter {
        connects,
        disconnects,
        sink,
        leaks_a_handle,
    })
}
