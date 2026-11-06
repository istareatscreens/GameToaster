use std::sync::mpsc::{self, RecvTimeoutError, Sender, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use super::messages::WorkerCommand;
use super::sessions::WatchedProgram;
use crate::gui::main_window::app::launch_monitor::{GoneTracker, GONE_AFTER};
use crate::launcher_gui::model::GameId;

const LOOK_EVERY: Duration = Duration::from_secs(1);

pub(super) struct ProgramWatch {
    _stop: Sender<()>,
}

impl ProgramWatch {
    pub(super) fn start(
        game_id: GameId,
        program: WatchedProgram,
        sender: SyncSender<WorkerCommand>,
    ) -> Result<Self, String> {
        Self::start_with(game_id, program, sender, GONE_AFTER)
    }

    pub(super) fn start_with(
        game_id: GameId,
        program: WatchedProgram,
        sender: SyncSender<WorkerCommand>,
        gone_after: Duration,
    ) -> Result<Self, String> {
        let (stop, stopped) = mpsc::channel::<()>();
        thread::Builder::new()
            .name(format!("gametoaster-program-watch-{}", game_id.0))
            .spawn(move || {
                let mut tracker = GoneTracker::new(gone_after);

                tracker.observe(true, Instant::now());
                loop {
                    match stopped.recv_timeout(LOOK_EVERY) {
                        Err(RecvTimeoutError::Timeout) => {}
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
                    }
                    let Some(presence) = tracker.observe(program.is_running(), Instant::now())
                    else {
                        continue;
                    };
                    if sender
                        .send(WorkerCommand::ProgramPresence { game_id, presence })
                        .is_err()
                    {
                        return;
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self { _stop: stop })
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/app/worker/program_watch__tests.rs"]
mod tests;
