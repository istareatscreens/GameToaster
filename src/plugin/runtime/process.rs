use std::ffi::{c_void, OsString};
use std::fs::File;
use std::mem::size_of;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    CreateProcessW, WaitForSingleObject, CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT,
    PROCESS_INFORMATION, STARTUPINFOW,
};

use crate::support::paths::same_file;
use crate::support::process::console_policy::{self, ConsoleSettings};
use crate::support::process::environment::environment_block;
use crate::support::process::handle::{creation_time, CreationTime, ProcessHandle};
use crate::support::process::image::ProgramKind;
use crate::support::process::invocation::{invocation, Invocation};
use crate::support::process::tree::direct_children;
use crate::support::process::ENDED_BY_LAUNCHER;
use crate::support::win::wide;

const END_WAIT_MS: u32 = 10_000;

pub struct StartSpec<'a> {
    pub executable: &'a Path,
    pub arguments: &'a [OsString],
    pub working_directory: &'a Path,
    pub environment: &'a [(OsString, OsString)],
    pub console: ConsoleSettings,
}

pub struct PluginProcess {
    root: Root,
    created: CreationTime,

    wrapped: Option<Wrapped>,
}

enum Root {
    Captured(Child),

    Console(ProcessHandle),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramState {
    Running,
    Ended,

    Unseen,
}

struct Wrapped {
    program: PathBuf,
    kind: ProgramKind,

    seen: Option<ProcessHandle>,
}

impl Wrapped {
    fn executable_state(&mut self, wrapper: u32, wrapper_created: CreationTime) -> ProgramState {
        if self.seen.is_none() {
            self.seen = direct_children(wrapper, wrapper_created)
                .into_iter()
                .find(|child| {
                    child
                        .image_path()
                        .is_some_and(|image| same_file(&image, &self.program))
                });
        }
        match &self.seen {
            Some(program) if program.has_exited() => ProgramState::Ended,
            Some(_) => ProgramState::Running,
            None => ProgramState::Unseen,
        }
    }
}

impl PluginProcess {
    pub fn start(
        spec: StartSpec,
        capture: impl FnOnce() -> Result<(File, File), String>,
    ) -> Result<Self, String> {
        let kind = ProgramKind::of(spec.executable);
        if spec.console.hides_consoles() || kind == ProgramKind::Graphical {
            return Self::start_hidden(spec, capture()?);
        }
        let plan = console_policy::plan(kind, spec.console);
        let Invocation {
            application,
            mut command_line,
        } = invocation(spec.executable, spec.arguments, plan)?;
        let application = wide(&application);
        let working_directory = wide(spec.working_directory);
        let environment = environment_block(spec.environment);
        let startup = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let mut information = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                PCWSTR(application.as_ptr()),
                PWSTR(command_line.as_mut_ptr()),
                None,
                None,
                false,
                plan.window.creation_flags() | CREATE_UNICODE_ENVIRONMENT,
                Some(environment.as_ptr() as *const c_void),
                PCWSTR(working_directory.as_ptr()),
                &startup,
                &mut information,
            )
        }
        .map_err(|error| error.to_string())?;
        unsafe {
            let _ = CloseHandle(information.hThread);
        }
        let root = ProcessHandle::from_owned(information.hProcess, information.dwProcessId);
        Ok(Self {
            created: root.created().unwrap_or_default(),
            root: Root::Console(root),
            wrapped: plan.pause_on_failure.then(|| Wrapped {
                program: spec.executable.to_path_buf(),
                kind,
                seen: None,
            }),
        })
    }

    fn start_hidden(spec: StartSpec, (stdout, stderr): (File, File)) -> Result<Self, String> {
        let mut command = Command::new(spec.executable);
        command
            .args(spec.arguments)
            .current_dir(spec.working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .creation_flags(CREATE_NO_WINDOW.0);
        for (key, value) in spec.environment {
            command.env(key, value);
        }
        let child = command.spawn().map_err(|error| error.to_string())?;
        Ok(Self {
            created: creation_time(HANDLE(child.as_raw_handle())).unwrap_or_default(),
            root: Root::Captured(child),
            wrapped: None,
        })
    }

    pub fn id(&self) -> u32 {
        match &self.root {
            Root::Captured(child) => child.id(),
            Root::Console(root) => root.pid(),
        }
    }

    pub fn created(&self) -> CreationTime {
        self.created
    }

    pub fn handle(&self) -> HANDLE {
        match &self.root {
            Root::Captured(child) => HANDLE(child.as_raw_handle()),
            Root::Console(root) => root.raw(),
        }
    }

    pub fn captures_output(&self) -> bool {
        matches!(self.root, Root::Captured(_))
    }

    pub fn exit_code(&mut self) -> Result<i32, String> {
        match &mut self.root {
            Root::Captured(child) => child
                .wait()
                .map(|status| status.code().unwrap_or(-1))
                .map_err(|error| error.to_string()),
            Root::Console(root) => root
                .exit_code()
                .ok_or_else(|| "the plugin process has not ended".to_string()),
        }
    }

    pub fn kill(&mut self) {
        match &mut self.root {
            Root::Captured(child) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Root::Console(root) => {
                root.terminate(ENDED_BY_LAUNCHER);
                let children = if self.wrapped.is_some() {
                    direct_children(root.pid(), self.created)
                } else {
                    Vec::new()
                };
                for child in &children {
                    child.terminate(ENDED_BY_LAUNCHER);
                }

                for process in std::iter::once(&*root).chain(&children) {
                    unsafe {
                        let _ = WaitForSingleObject(process.raw(), END_WAIT_MS);
                    }
                }
            }
        }
    }

    pub fn program_state(&mut self) -> ProgramState {
        if let (Root::Console(root), Some(wrapped)) = (&self.root, &mut self.wrapped) {
            if wrapped.kind == ProgramKind::Console {
                return wrapped.executable_state(root.pid(), self.created);
            }
        }
        if self.has_exited() {
            ProgramState::Ended
        } else {
            ProgramState::Running
        }
    }

    fn has_exited(&self) -> bool {
        let wait = unsafe { WaitForSingleObject(self.handle(), 0) };
        wait == WAIT_OBJECT_0
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/runtime/process__tests.rs"]
mod tests;
