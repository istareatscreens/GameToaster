use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use eframe::egui;

use crate::game_launcher_screen::ControllerScreenConfig;

use super::monitor::{self, same_physical_monitor};

const POLL_INTERVAL: Duration = Duration::from_millis(750);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct MonitorTarget {
    stable_id: Option<String>,
    adapter: Option<u32>,
}

impl MonitorTarget {
    pub fn from_config(config: &ControllerScreenConfig) -> Option<Self> {
        if !config.enabled || !config.reopen_on_monitor_return {
            return None;
        }
        let stable_id = config
            .calibration
            .as_ref()
            .map(|calibration| calibration.monitor_id.trim().to_string())
            .filter(|id| !id.is_empty());
        (stable_id.is_some() || config.monitor_adapter.is_some()).then_some(Self {
            stable_id,
            adapter: config.monitor_adapter,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Presence {
    #[default]
    Unobserved,
    Present,
    Absent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PresenceEdge {
    Unavailable,
    Returned,
}

#[derive(Default)]
struct PresenceTracker {
    target: Option<MonitorTarget>,
    presence: Presence,
}

impl PresenceTracker {
    fn set_target(&mut self, target: Option<MonitorTarget>) {
        if self.target != target {
            self.target = target;
            self.presence = Presence::Unobserved;
        }
    }

    fn observe(&mut self, result: Result<bool, String>) -> Option<PresenceEdge> {
        let present = result.ok()?;
        let next = if present {
            Presence::Present
        } else {
            Presence::Absent
        };
        let edge = match (self.presence, next) {
            (Presence::Present, Presence::Absent) => Some(PresenceEdge::Unavailable),
            (Presence::Absent, Presence::Present) => Some(PresenceEdge::Returned),
            _ => None,
        };
        self.presence = next;
        edge
    }
}

enum Command {
    SetTarget(Option<MonitorTarget>),
    Stop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum MonitorWatchEvent {
    Unavailable(MonitorTarget),
    Returned(MonitorTarget),
}

pub(in crate::gui::game_launcher_screen) struct MonitorReturnWatcher {
    commands: Sender<Command>,
    events: Receiver<MonitorWatchEvent>,
    worker: Option<JoinHandle<()>>,
    presence: SharedPresence,
}

impl MonitorReturnWatcher {
    pub fn new(ctx: egui::Context) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let presence: SharedPresence = Arc::new(Mutex::new(None));
        let shared = Arc::clone(&presence);
        let worker = thread::Builder::new()
            .name("gametoaster-monitor-return".into())
            .spawn(move || run(command_rx, event_tx, ctx, shared));
        let worker = worker.ok();
        Self {
            commands: command_tx,
            events: event_rx,
            worker,
            presence,
        }
    }

    pub fn presence_of(&self, target: &MonitorTarget) -> Option<bool> {
        read_presence(&self.presence, target)
    }

    pub fn probe_error(&self, target: &MonitorTarget) -> Option<String> {
        read_probe_error(&self.presence, target)
    }

    #[cfg(test)]
    pub fn record_presence_for_test(&self, target: &MonitorTarget, present: bool) {
        record_observation(&self.presence, target, &Ok(present));
    }

    pub fn configure(&self, config: &ControllerScreenConfig) {
        let _ = self
            .commands
            .send(Command::SetTarget(MonitorTarget::from_config(config)));
    }

    pub fn poll(&self) -> Vec<MonitorWatchEvent> {
        self.events.try_iter().collect()
    }
}

impl Drop for MonitorReturnWatcher {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(
    commands: Receiver<Command>,
    events: Sender<MonitorWatchEvent>,
    ctx: egui::Context,
    presence: SharedPresence,
) {
    let mut tracker = PresenceTracker::default();
    loop {
        match commands.recv_timeout(POLL_INTERVAL) {
            Ok(command) => {
                if apply_command(&mut tracker, command) {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        loop {
            match commands.try_recv() {
                Ok(command) => {
                    if apply_command(&mut tracker, command) {
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let Some(target) = tracker.target.clone() else {
            continue;
        };
        let observation = target_present(&target);

        let changed = record_observation(&presence, &target, &observation);
        let event = match tracker.observe(observation) {
            Some(PresenceEdge::Unavailable) => Some(MonitorWatchEvent::Unavailable(target)),
            Some(PresenceEdge::Returned) => Some(MonitorWatchEvent::Returned(target)),
            None => None,
        };
        let wake = match event {
            Some(event) => events.send(event).is_ok(),
            None => changed,
        };
        if wake {
            ctx.request_repaint();
        }
    }
}

fn apply_command(tracker: &mut PresenceTracker, command: Command) -> bool {
    match command {
        Command::SetTarget(target) => {
            tracker.set_target(target);
            false
        }
        Command::Stop => true,
    }
}

type SharedPresence = Arc<Mutex<Option<(MonitorTarget, Result<bool, String>)>>>;

fn record_observation(
    shared: &SharedPresence,
    target: &MonitorTarget,
    observation: &Result<bool, String>,
) -> bool {
    let Ok(mut slot) = shared.lock() else {
        return false;
    };
    let changed = match slot.as_ref() {
        Some((observed, previous)) if observed == target => previous != observation,
        _ => true,
    };
    *slot = Some((target.clone(), observation.clone()));
    changed
}

fn read_presence(shared: &SharedPresence, target: &MonitorTarget) -> Option<bool> {
    let slot = shared.lock().ok()?;
    slot.as_ref()
        .filter(|(observed, _)| observed == target)
        .and_then(|(_, observation)| observation.as_ref().ok().copied())
}

fn read_probe_error(shared: &SharedPresence, target: &MonitorTarget) -> Option<String> {
    let slot = shared.lock().ok()?;
    slot.as_ref()
        .filter(|(observed, _)| observed == target)
        .and_then(|(_, observation)| observation.as_ref().err().cloned())
}

fn target_present(target: &MonitorTarget) -> Result<bool, String> {
    let monitors = monitor::enumerate()?;
    if let Some(stable_id) = target.stable_id.as_deref() {
        return Ok(monitors
            .iter()
            .any(|candidate| same_physical_monitor(&candidate.id.key(), stable_id)));
    }
    Ok(target
        .adapter
        .is_some_and(|adapter| monitors.iter().any(|monitor| monitor.index == adapter)))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/display/monitor_watch__tests.rs"]
mod tests;
