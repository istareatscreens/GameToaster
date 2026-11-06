use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use windows::Win32::Foundation::HANDLE;

use crate::launcher_gui::model::GameEntry;
use crate::launcher_gui::process::{ProcessManager, SpawnSpec, SpawnedProcess};
use crate::support::process::console_policy::{self, ConsolePlan, ConsoleSettings};
use crate::support::process::handle::{creation_time, CreationTime};
use crate::support::process::image::ProgramKind;
use crate::support::process::interpreter::script_command;

mod cancel;
mod handoff;
mod pre_start;

pub use cancel::LaunchCancel;
pub use handoff::HandoffGate;
pub use pre_start::{PreStartFailure, PRE_START_TIMEOUT};

#[derive(Clone, Debug)]
pub struct LaunchPlan {
    pub root_executable: PathBuf,
    pub game_directory: PathBuf,
    pub console: ConsolePlan,
}

#[derive(Clone)]
pub struct RuntimeService {
    launcher_dir: PathBuf,
}

impl RuntimeService {
    pub fn new(launcher_dir: PathBuf) -> Self {
        Self { launcher_dir }
    }

    fn prepare(&self, game: &GameEntry, settings: ConsoleSettings) -> Result<LaunchPlan, String> {
        let launch_target = game.launch_target();
        if !launch_target.is_file() {
            return Err(format!(
                "resolved launch target is missing: {}",
                launch_target.display()
            ));
        }
        let game_directory = game.game_dir()?.to_path_buf();
        Ok(LaunchPlan {
            root_executable: launch_target.to_path_buf(),
            game_directory,
            console: console_policy::plan(ProgramKind::of(launch_target), settings),
        })
    }

    pub fn run_close_script(&self, script: &Path) -> Result<(u32, CreationTime), String> {
        if !script.is_file() {
            return Err(format!("close script is missing: {}", script.display()));
        }
        let directory = script.parent().unwrap_or(self.launcher_dir.as_path());
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let child = script_command(script, &[])?
            .current_dir(directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("failed to run close script {}: {error}", script.display()))?;

        let created = creation_time(HANDLE(child.as_raw_handle())).unwrap_or_default();
        Ok((child.id(), created))
    }

    pub fn launch(
        &self,
        game: &GameEntry,
        settings: ConsoleSettings,
        processes: &mut ProcessManager,
    ) -> Result<SpawnedProcess, String> {
        let plan = self.prepare(game, settings)?;
        processes.spawn(
            game.id,
            &SpawnSpec {
                executable: plan.root_executable,
                working_directory: plan.game_directory,
                arguments: Vec::new(),
                console: plan.console,
                environment: Vec::new(),
            },
        )
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/runtime/mod__tests.rs"]
mod tests;
