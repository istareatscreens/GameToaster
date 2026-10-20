use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;

use crate::game_launcher_screen::ControllerConfig;
use crate::gui::game_launcher_screen::domain::command::LauncherCommand;

use super::adapter::ControllerSession;
use super::audit::{ReleaseProof, ResourceAudit};
use super::buttons::{Button, ButtonWord};
use super::edges::EdgeTracker;
use super::events::{EventSink, SessionEvent};
use super::lights::LightsRequest;
use super::registry::AdapterRegistry;

const EVENT_CAPACITY: usize = 64;
pub(in crate::gui::game_launcher_screen) const RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum ManagerEvent {
    Connected,
    Snapshot(ButtonWord),
    Edge { button: Button, pressed: bool },
    Error(String),
    Disconnected,
}

pub(in crate::gui::game_launcher_screen) struct ControllerManager {
    registry: AdapterRegistry,
    session: Option<Box<dyn ControllerSession>>,
    events: Option<Receiver<SessionEvent>>,
    config: Option<ControllerConfig>,
    edge_tracker: EdgeTracker,
    needs_baseline: bool,
    connected: bool,

    ctx: Option<egui::Context>,

    audit: Arc<ResourceAudit>,
}

impl Default for ControllerManager {
    fn default() -> Self {
        Self::new(AdapterRegistry::with_default_adapters())
    }
}

impl ControllerManager {
    fn new(registry: AdapterRegistry) -> Self {
        Self {
            registry,
            session: None,
            events: None,
            config: None,
            edge_tracker: EdgeTracker::default(),
            needs_baseline: true,
            connected: false,
            ctx: None,
            audit: ResourceAudit::new(),
        }
    }

    pub fn set_context(&mut self, ctx: egui::Context) {
        self.ctx = Some(ctx);
    }

    pub fn is_active(&self) -> bool {
        self.session.is_some()
    }

    pub fn connect(&mut self, config: &ControllerConfig) -> Result<(), String> {
        self.registry.validate(config)?;
        if self.session.is_some() {
            if self.config.as_ref() == Some(config) {
                return Ok(());
            }
            self.disconnect(RELEASE_TIMEOUT)?;
        }
        let adapter = self
            .registry
            .get(&config.driver_id)
            .ok_or_else(|| format!("controller driver '{}' is not registered", config.driver_id))?;
        let (sender, receiver) = mpsc::sync_channel(EVENT_CAPACITY);

        let sink = EventSink::new(sender, self.ctx.clone());
        let audit = ResourceAudit::new();
        let session = adapter.connect(config, sink, audit.clone())?;
        self.session = Some(session);
        self.audit = audit;
        self.events = Some(receiver);
        self.config = Some(config.clone());
        self.edge_tracker.disconnect();
        self.needs_baseline = true;
        self.connected = false;
        Ok(())
    }

    pub fn poll(&mut self) -> Vec<ManagerEvent> {
        let mut input = Vec::new();
        if let Some(events) = &self.events {
            while let Ok(event) = events.try_recv() {
                input.push(event);
            }
        }
        let mut output = Vec::new();
        for event in input {
            match event {
                SessionEvent::Connected => {
                    self.connected = true;
                    self.needs_baseline = true;
                    output.push(ManagerEvent::Connected);
                }
                SessionEvent::Snapshot(snapshot) => {
                    output.push(ManagerEvent::Snapshot(snapshot));
                    if self.needs_baseline {
                        self.edge_tracker.establish_baseline(snapshot);
                        self.needs_baseline = false;
                        continue;
                    }
                    let edges = self.edge_tracker.update(snapshot);
                    for button in Button::ALL {
                        if edges.rising.contains(button) {
                            output.push(ManagerEvent::Edge {
                                button,
                                pressed: true,
                            });
                        }
                        if edges.falling.contains(button) {
                            output.push(ManagerEvent::Edge {
                                button,
                                pressed: false,
                            });
                        }
                    }
                }
                SessionEvent::Error(error) => output.push(ManagerEvent::Error(error)),
                SessionEvent::Disconnected => {
                    self.connected = false;
                    let edges = self.edge_tracker.disconnect();
                    for button in Button::ALL {
                        if edges.falling.contains(button) {
                            output.push(ManagerEvent::Edge {
                                button,
                                pressed: false,
                            });
                        }
                    }
                    output.push(ManagerEvent::Disconnected);
                }
            }
        }
        output
    }

    pub fn disconnect(&mut self, timeout: Duration) -> Result<ReleaseProof, String> {
        let Some(session) = self.session.as_mut() else {
            return Ok(ReleaseProof::default());
        };
        session.disconnect(timeout)?;
        let proof = self.audit.proof();
        if proof.open_handles != 0 || proof.live_readers != 0 {
            return Err(format!(
                "controller release proof failed: {} handle(s), {} reader(s) remain",
                proof.open_handles, proof.live_readers
            ));
        }
        self.session = None;
        self.events = None;
        self.config = None;
        self.connected = false;
        self.needs_baseline = true;
        self.edge_tracker.disconnect();
        Ok(proof)
    }

    pub fn command_for(&self, button: Button) -> Option<LauncherCommand> {
        let config = self.config.as_ref()?;
        self.registry
            .get(&config.driver_id)?
            .descriptor()
            .command_for(button)
    }

    pub fn lights_request<'a>(&self, config: &'a ControllerConfig) -> Option<LightsRequest<'a>> {
        self.registry.lights_request(config)
    }

    #[cfg(test)]
    pub fn resource_counts(&self) -> (usize, usize) {
        self.audit.counts()
    }
}

impl Drop for ControllerManager {
    fn drop(&mut self) {
        let _ = self.disconnect(RELEASE_TIMEOUT);
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/manager__tests.rs"]
mod tests;
