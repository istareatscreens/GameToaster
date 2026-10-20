use std::collections::BTreeMap;
use std::sync::Arc;

use crate::launcher_gui::model::GameId;

mod console;
mod discovery;
mod escaped_watch;
mod job;
mod spawn;
mod window;

pub use console::hide_console_for_pid;
pub use discovery::{
    find_pid_by_name, find_pid_by_path, is_process_path_running, is_process_running, pids_by_name,
    pids_by_path, terminate_by_name, terminate_by_path, ProcessWatch,
};
pub(crate) use escaped_watch::{EscapedProcessWatch, WatchEvent};
pub use window::has_visible_window;

use job::ManagedJob;
use spawn::validate_spawn_spec;

use crate::support::process::console_policy::ConsolePlan;
use crate::support::process::handle::CreationTime;

#[derive(Clone, Debug)]
pub struct SpawnSpec {
    pub executable: std::path::PathBuf,
    pub working_directory: std::path::PathBuf,
    pub arguments: Vec<std::ffi::OsString>,
    pub console: ConsolePlan,
    pub environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpawnedProcess {
    pub pid: u32,
    pub created: CreationTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessCompletion {
    Exited(GameId),
    Failed { message: String },
}

pub struct ProcessManager {
    jobs: BTreeMap<GameId, ManagedJob>,
    completion_sink: Arc<dyn Fn(ProcessCompletion) + Send + Sync>,
}

impl ProcessManager {
    pub fn with_completion_sink(
        completion_sink: Arc<dyn Fn(ProcessCompletion) + Send + Sync>,
    ) -> Self {
        Self {
            jobs: BTreeMap::new(),
            completion_sink,
        }
    }

    pub fn spawn(&mut self, game_id: GameId, spec: &SpawnSpec) -> Result<SpawnedProcess, String> {
        self.stop_game(game_id)?;
        validate_spawn_spec(spec)?;
        let job = ManagedJob::spawn(game_id, spec, self.completion_sink.clone())?;
        let spawned = SpawnedProcess {
            pid: job.pid,
            created: job.created,
        };
        self.jobs.insert(game_id, job);
        Ok(spawned)
    }

    pub fn stop_game(&mut self, game_id: GameId) -> Result<(), String> {
        if let Some(mut job) = self.jobs.remove(&game_id) {
            job.terminate_and_wait()?;
        }
        Ok(())
    }

    pub fn release_completed(&mut self, game_id: GameId) -> bool {
        let Some(job) = self.jobs.remove(&game_id) else {
            return false;
        };
        job.release_completed();
        true
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        let mut failures = Vec::new();
        let jobs = std::mem::take(&mut self.jobs);
        for (game_id, mut job) in jobs {
            if let Err(error) = job.terminate_and_wait() {
                failures.push(format!("game {game_id}: {error}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/process/mod__tests.rs"]
mod tests;
