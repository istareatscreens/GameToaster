mod catalog;
mod close;
mod context;
mod launch;
mod library;
mod messages;
mod pickers;
mod plugin_actions;
mod plugin_backend;
mod plugin_runtime;
mod sessions;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};

use eframe::egui;
use rust_i18n::t;

use crate::launcher_gui::logging::LauncherLog;
use context::WorkerContext;
pub(super) use messages::{WorkerCommand, WorkerResult};

pub(super) struct Worker {
    sender: SyncSender<WorkerCommand>,
    pub(super) receiver: Receiver<WorkerResult>,
    thread: Option<JoinHandle<()>>,
    shutdown_sent: bool,
}

impl Worker {
    pub(super) fn start(
        launcher_dir: PathBuf,
        log: LauncherLog,
        ctx: egui::Context,
        game_state: crate::gui::common::game_state::GameStateHub,
    ) -> Self {
        let (sender, commands) = mpsc::sync_channel::<WorkerCommand>(64);
        let internal_sender = sender.clone();
        let (results, receiver) = mpsc::channel::<WorkerResult>();
        let thread = thread::Builder::new()
            .name("gametoaster-launcher-worker".into())
            .spawn(move || {
                WorkerContext::new(launcher_dir, log, ctx, game_state, results, internal_sender)
                    .run(commands)
            })
            .expect("failed to create bounded launcher worker");
        Self {
            sender,
            receiver,
            thread: Some(thread),
            shutdown_sent: false,
        }
    }

    pub(super) fn try_send(&self, command: WorkerCommand) -> Result<(), String> {
        self.sender.try_send(command).map_err(|error| match error {
            TrySendError::Full(_) => t!("errors.worker_busy").to_string(),
            TrySendError::Disconnected(_) => t!("errors.worker_stopped").to_string(),
        })
    }

    pub(super) fn shutdown(&mut self) -> Result<(), String> {
        if self.shutdown_sent {
            return Ok(());
        }
        self.sender
            .try_send(WorkerCommand::Shutdown)
            .map_err(|_| t!("errors.shutdown_not_queued").to_string())?;
        self.shutdown_sent = true;
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if !self.shutdown_sent {
            let _ = self.sender.send(WorkerCommand::Shutdown);
            self.shutdown_sent = true;
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
