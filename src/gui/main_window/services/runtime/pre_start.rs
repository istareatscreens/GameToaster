use std::fs::{self, File};
use std::path::Path;
use std::time::Duration;

use rust_i18n::t;
use windows::Win32::Foundation::{HANDLE, WAIT_EVENT, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{WaitForMultipleObjects, WaitForSingleObject, INFINITE};

use super::cancel::LaunchCancel;
use super::RuntimeService;
use crate::launcher_gui::model::GameId;
use crate::plugin::runtime::host::output_tail;
use crate::plugin::runtime::process::{PluginProcess, ProgramState, StartSpec};
use crate::support::process::console_policy::ConsoleSettings;
use crate::support::process::handle::CreationTime;

pub const PRE_START_TIMEOUT: Duration = Duration::from_secs(10 * 60);

const OUTPUT_FOLDER: &str = "script_output";

const OUTPUT_TAIL_LIMIT: u64 = 4 * 1024;

const LOOK_AGAIN_MS: u32 = 250;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreStartFailure {
    NotStarted(String),

    Failed { code: i32, output: Option<String> },

    TimedOut,

    Cancelled,
}

impl PreStartFailure {
    pub fn message(&self, script: &Path) -> String {
        let script = script.display().to_string();
        match self {
            Self::NotStarted(detail) => {
                t!("errors.pre_start_not_started", script => script, detail => detail).to_string()
            }
            Self::Failed { code, output } => {
                let mut message =
                    t!("errors.pre_start_failed", script => script, code => code).to_string();
                if let Some(output) = output {
                    message.push_str("\n\n");
                    message.push_str(output);
                }
                message
            }
            Self::TimedOut => t!(
                "errors.pre_start_timeout",
                script => script,
                minutes => PRE_START_TIMEOUT.as_secs() / 60
            )
            .to_string(),
            Self::Cancelled => t!("errors.launch_cancelled").to_string(),
        }
    }
}

impl RuntimeService {
    pub fn run_pre_start_script(
        &self,
        game_id: GameId,
        script: &Path,
        console: ConsoleSettings,
        timeout: Duration,
        cancel: &LaunchCancel,
        started: impl FnOnce(u32, CreationTime),
    ) -> Result<(), PreStartFailure> {
        if cancel.is_cancelled() {
            return Err(PreStartFailure::Cancelled);
        }
        if !script.is_absolute() || !script.is_file() {
            return Err(PreStartFailure::NotStarted(format!(
                "{} is missing or not an absolute path",
                script.display()
            )));
        }
        let directory = script.parent().unwrap_or(self.launcher_dir.as_path());
        let output_folder = self.launcher_dir.join(OUTPUT_FOLDER);
        let stdout_path = output_folder.join(format!("{game_id}-pre-start.out.txt"));
        let stderr_path = output_folder.join(format!("{game_id}-pre-start.err.txt"));
        let mut process = PluginProcess::start(
            StartSpec {
                executable: script,
                arguments: &[],
                working_directory: directory,
                environment: &[],
                console,
            },
            || capture_files(&output_folder, &stdout_path, &stderr_path),
        )
        .map_err(PreStartFailure::NotStarted)?;
        started(process.id(), process.created());
        let code = wait(&mut process, timeout, cancel)?;
        if code == 0 {
            return Ok(());
        }
        Err(PreStartFailure::Failed {
            code,
            output: process
                .captures_output()
                .then(|| output_tail(&stdout_path, &stderr_path, OUTPUT_TAIL_LIMIT))
                .flatten(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Waited {
    Ended,
    TimedOut,
    Cancelled,
    Failed(u32),
}

fn wait_for(process: HANDLE, cancel: &LaunchCancel, milliseconds: u32) -> Waited {
    let result: WAIT_EVENT = match cancel.event() {
        Some(event) => unsafe { WaitForMultipleObjects(&[process, event], false, milliseconds) },
        None => unsafe { WaitForSingleObject(process, milliseconds) },
    };
    if result == WAIT_OBJECT_0 {
        Waited::Ended
    } else if result.0 == WAIT_OBJECT_0.0 + 1 {
        Waited::Cancelled
    } else if result == WAIT_TIMEOUT {
        Waited::TimedOut
    } else {
        Waited::Failed(result.0)
    }
}

fn wait(
    process: &mut PluginProcess,
    timeout: Duration,
    cancel: &LaunchCancel,
) -> Result<i32, PreStartFailure> {
    let handle = process.handle();
    let deadline_ms = timeout.as_millis().min(u128::from(u32::MAX - 1)) as u32;
    let mut waited = wait_for(handle, cancel, deadline_ms);
    while waited == Waited::TimedOut {
        waited = match process.program_state() {
            ProgramState::Running => break,
            ProgramState::Ended => wait_for(handle, cancel, INFINITE),
            ProgramState::Unseen => wait_for(handle, cancel, LOOK_AGAIN_MS),
        };
    }
    match waited {
        Waited::Ended => process.exit_code().map_err(PreStartFailure::NotStarted),
        Waited::TimedOut => {
            end(process);
            Err(PreStartFailure::TimedOut)
        }
        Waited::Cancelled => {
            end(process);
            Err(PreStartFailure::Cancelled)
        }
        Waited::Failed(result) => {
            end(process);
            Err(PreStartFailure::NotStarted(format!(
                "waiting for the script failed (wait result {result})"
            )))
        }
    }
}

fn end(process: &mut PluginProcess) {
    process.kill();
}

fn capture_files(folder: &Path, stdout: &Path, stderr: &Path) -> Result<(File, File), String> {
    fs::create_dir_all(folder)
        .map_err(|error| format!("failed to create {}: {error}", folder.display()))?;
    let create = |path: &Path| {
        File::create(path).map_err(|error| format!("failed to create {}: {error}", path.display()))
    };
    Ok((create(stdout)?, create(stderr)?))
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/runtime/pre_start__tests.rs"]
mod tests;
