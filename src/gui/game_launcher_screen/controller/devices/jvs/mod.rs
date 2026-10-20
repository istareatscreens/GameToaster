use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::game_launcher_screen::ControllerConfig;

use super::super::framework::adapter::{ControllerAdapter, ControllerSession};
use super::super::framework::audit::ResourceAudit;
use super::super::framework::descriptor::{
    required_option, ControllerChoice, ControllerDriverDescriptor, ControllerOptionDescriptor,
    ControllerOptionKind,
};
use super::super::framework::events::EventSink;
use super::super::framework::lights::TURN_OFF_ON_GAME_CLOSE;
use super::super::transport::handles::ReaderGuard;
use super::led_board;
use super::ring_bindings::RING_BINDINGS;
use host::{JvsHost, JvsPlayer};
use link::JvsTransport;

mod host;
mod link;
mod mxjvs;
mod protocol;
mod reader;

const PLAYER_CHOICES: &[ControllerChoice] = &[
    ControllerChoice {
        value: "p1",
        label_key: "controller.type.player1",
    },
    ControllerChoice {
        value: "p2",
        label_key: "controller.type.player2",
    },
];
const OPTIONS: &[ControllerOptionDescriptor] = &[
    ControllerOptionDescriptor {
        key: "player",
        label_key: "controller.type.jvs_player",
        default: "p1",
        kind: ControllerOptionKind::Choice(PLAYER_CHOICES),
    },
    ControllerOptionDescriptor {
        key: "endpoint",
        label_key: "controller.type.jvs_endpoint",
        default: r"\\.\mxjvs",
        kind: ControllerOptionKind::Text { allow_empty: false },
    },
    led_board::ports_option(""),
    TURN_OFF_ON_GAME_CLOSE,
];
pub(super) static DESCRIPTOR: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "jvs",
    label_key: "controller.type.jvs",
    recommended_layout: "washing-machine",
    options: OPTIONS,
    lights: Some(led_board::DESCRIPTOR),
    bindings: RING_BINDINGS,
};

#[derive(Default)]
pub(in crate::gui::game_launcher_screen::controller) struct JvsAdapter;

impl ControllerAdapter for JvsAdapter {
    fn descriptor(&self) -> &'static ControllerDriverDescriptor {
        &DESCRIPTOR
    }

    fn validate(&self, config: &ControllerConfig) -> Result<(), String> {
        DESCRIPTOR.validate(config)?;

        link::normalize_endpoint(required_option(config, "endpoint")?)?;
        Ok(())
    }

    fn connect(
        &self,
        config: &ControllerConfig,
        events: EventSink,
        audit: Arc<ResourceAudit>,
    ) -> Result<Box<dyn ControllerSession>, String> {
        self.validate(config)?;
        let transport = JvsTransport::open(required_option(config, "endpoint")?, &audit)?;
        let player = JvsPlayer::parse(required_option(config, "player")?)?;
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let thread_transport = transport.clone();
        let reader = thread::Builder::new()
            .name("gametoaster-controller-jvs".into())
            .spawn(move || {
                let _guard = ReaderGuard::new(&audit);
                let mut link = thread_transport.link();
                reader::run(&JvsHost { player }, link.as_mut(), &events);
                let _ = done_tx.try_send(());
            })
            .map_err(|error| format!("failed to start JVS reader: {error}"))?;
        Ok(Box::new(JvsSession {
            transport: Some(transport),
            reader: Some(reader),
            done: done_rx,
        }))
    }
}

struct JvsSession {
    transport: Option<JvsTransport>,
    reader: Option<JoinHandle<()>>,
    done: mpsc::Receiver<()>,
}

impl ControllerSession for JvsSession {
    fn disconnect(&mut self, timeout: Duration) -> Result<(), String> {
        if let Some(transport) = &self.transport {
            transport.cancel();
        }
        if self.reader.is_some() {
            self.done.recv_timeout(timeout).map_err(|_| {
                "JVS reader did not acknowledge cancellation within the handoff timeout".to_string()
            })?;
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.transport = None;
        Ok(())
    }
}

impl Drop for JvsSession {
    fn drop(&mut self) {
        if let Some(transport) = &self.transport {
            transport.cancel();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
