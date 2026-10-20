use std::ffi::c_void;
use std::mem::size_of;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectAssociateCompletionPortInformation,
    JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
    JOBOBJECT_ASSOCIATE_COMPLETION_PORT, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{
    CreateProcessW, ResumeThread, TerminateProcess, WaitForSingleObject, INFINITE,
    PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
};
use windows::Win32::System::IO::{
    CreateIoCompletionPort, GetQueuedCompletionStatus, PostQueuedCompletionStatus, OVERLAPPED,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

use super::spawn::creation_flags;
use super::{ProcessCompletion, SpawnSpec};
use crate::support::process::environment::environment_block;
use crate::support::process::handle::{creation_time, CreationTime};
use crate::support::process::invocation::{invocation, Invocation};
use crate::support::process::ENDED_BY_LAUNCHER;
use crate::support::win::wide;

pub(super) struct ManagedJob {
    job: HANDLE,
    completion_port: HANDLE,
    process: HANDLE,
    pub(super) pid: u32,
    pub(super) created: CreationTime,
    terminated: bool,
    completion_watcher: Option<JoinHandle<()>>,
}

impl ManagedJob {
    pub(super) fn spawn(
        game_id: crate::launcher_gui::model::GameId,
        spec: &SpawnSpec,
        completion_sink: Arc<dyn Fn(ProcessCompletion) + Send + Sync>,
    ) -> Result<Self, String> {
        let Invocation {
            application,
            mut command_line,
        } = invocation(&spec.executable, &spec.arguments, spec.console)?;
        let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|error| format!("failed to create process Job Object: {error}"))?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if let Err(error) = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } {
            unsafe {
                let _ = CloseHandle(job);
            }
            return Err(format!(
                "failed to configure kill-on-close Job Object: {error}"
            ));
        }

        let completion_port =
            match unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, None, 0, 1) } {
                Ok(port) => port,
                Err(error) => {
                    unsafe {
                        let _ = CloseHandle(job);
                    }
                    return Err(format!("failed to create Job completion port: {error}"));
                }
            };
        let association = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
            CompletionKey: job.0,
            CompletionPort: completion_port,
        };
        if let Err(error) = unsafe {
            SetInformationJobObject(
                job,
                JobObjectAssociateCompletionPortInformation,
                &association as *const _ as *const c_void,
                size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
            )
        } {
            unsafe {
                let _ = CloseHandle(completion_port);
                let _ = CloseHandle(job);
            }
            return Err(format!(
                "failed to associate the Job completion port: {error}"
            ));
        }

        let executable = wide(&application);
        let cwd = wide(&spec.working_directory);
        let environment = environment_block(&spec.environment);
        let mut startup = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };

        if spec.console.window.starts_hidden() {
            startup.dwFlags |= STARTF_USESHOWWINDOW;
            startup.wShowWindow = SW_HIDE.0 as u16;
        }
        let mut information = PROCESS_INFORMATION::default();
        let created = unsafe {
            CreateProcessW(
                PCWSTR(executable.as_ptr()),
                PWSTR(command_line.as_mut_ptr()),
                None,
                None,
                false,
                creation_flags(spec.console.window),
                Some(environment.as_ptr() as *const c_void),
                PCWSTR(cwd.as_ptr()),
                &startup,
                &mut information,
            )
        };
        if let Err(error) = created {
            unsafe {
                let _ = CloseHandle(completion_port);
                let _ = CloseHandle(job);
            }
            return Err(format!(
                "failed to create {} suspended: {error}",
                spec.executable.display()
            ));
        }

        if let Err(error) = unsafe { AssignProcessToJobObject(job, information.hProcess) } {
            unsafe {
                let _ = TerminateProcess(information.hProcess, 1);
                let _ = WaitForSingleObject(information.hProcess, 5_000);
                let _ = CloseHandle(information.hThread);
                let _ = CloseHandle(information.hProcess);
                let _ = CloseHandle(completion_port);
                let _ = CloseHandle(job);
            }
            return Err(format!(
                "created {} suspended but could not safely assign it to the launcher Job Object: {error}",
                spec.executable.display()
            ));
        }

        let resume_result = unsafe { ResumeThread(information.hThread) };
        unsafe {
            let _ = CloseHandle(information.hThread);
        }
        if resume_result == u32::MAX {
            unsafe {
                let _ = TerminateJobObject(job, 1);
                let _ = WaitForSingleObject(information.hProcess, 5_000);
                let _ = CloseHandle(information.hProcess);
                let _ = CloseHandle(completion_port);
                let _ = CloseHandle(job);
            }
            return Err(format!(
                "failed to resume {} after Job assignment",
                spec.executable.display()
            ));
        }

        let raw_completion_port = completion_port.0 as usize;
        let expected_key = job.0 as usize;
        let completion_watcher = match thread::Builder::new()
            .name(format!("gametoaster-game-wait-{}", information.dwProcessId))
            .spawn(move || {
                let port = HANDLE(raw_completion_port as *mut c_void);
                loop {
                    let mut message = 0u32;
                    let mut key = 0usize;
                    let mut overlapped: *mut OVERLAPPED = std::ptr::null_mut();
                    match unsafe {
                        GetQueuedCompletionStatus(
                            port,
                            &mut message,
                            &mut key,
                            &mut overlapped,
                            INFINITE,
                        )
                    } {
                        Ok(())
                            if message == JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO
                                && key == expected_key =>
                        {
                            completion_sink(ProcessCompletion::Exited(game_id));
                            break;
                        }
                        Ok(()) if message == WATCHER_CANCEL_MESSAGE => break,
                        Ok(()) => continue,
                        Err(error) => {
                            completion_sink(ProcessCompletion::Failed {
                                message: format!("Job completion wait failed: {error}"),
                            });
                            break;
                        }
                    }
                }
            }) {
            Ok(watcher) => watcher,
            Err(error) => {
                unsafe {
                    let _ = TerminateJobObject(job, 1);
                    let _ = WaitForSingleObject(information.hProcess, 5_000);
                    let _ = CloseHandle(information.hProcess);
                    let _ = CloseHandle(completion_port);
                    let _ = CloseHandle(job);
                }
                return Err(format!("failed to start game completion watcher: {error}"));
            }
        };

        Ok(Self {
            job,
            completion_port,
            process: information.hProcess,
            pid: information.dwProcessId,

            created: creation_time(information.hProcess).unwrap_or_default(),
            terminated: false,
            completion_watcher: Some(completion_watcher),
        })
    }

    pub(super) fn release_completed(mut self) {
        self.terminated = true;
        self.join_completion_watcher(false);
    }

    pub(super) fn terminate_and_wait(&mut self) -> Result<(), String> {
        if self.terminated {
            return Ok(());
        }
        unsafe { TerminateJobObject(self.job, ENDED_BY_LAUNCHER) }
            .map_err(|error| format!("failed to terminate process Job Object: {error}"))?;

        if !wait_job_drained(self.job, 10_000) {
            return Err(format!(
                "timed out waiting for managed Job (root PID {}) to drain",
                self.pid
            ));
        }
        self.terminated = true;
        self.join_completion_watcher(true);
        Ok(())
    }

    fn join_completion_watcher(&mut self, cancel: bool) {
        if cancel && self.completion_watcher.is_some() {
            unsafe {
                let _ = PostQueuedCompletionStatus(
                    self.completion_port,
                    WATCHER_CANCEL_MESSAGE,
                    0,
                    None,
                );
            }
        }
        if let Some(watcher) = self.completion_watcher.take() {
            let _ = watcher.join();
        }
    }
}

fn job_is_drained(job: HANDLE) -> bool {
    let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
    unsafe {
        QueryInformationJobObject(
            job,
            JobObjectBasicAccountingInformation,
            &mut info as *mut _ as *mut c_void,
            size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            None,
        )
    }
    .is_err()
        || info.ActiveProcesses == 0
}

fn wait_job_drained(job: HANDLE, timeout_ms: u64) -> bool {
    let start = Instant::now();
    loop {
        if job_is_drained(job) {
            return true;
        }
        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

impl Drop for ManagedJob {
    fn drop(&mut self) {
        if !self.terminated {
            unsafe {
                let _ = TerminateJobObject(self.job, ENDED_BY_LAUNCHER);
            }
            let _ = wait_job_drained(self.job, 10_000);
        }
        self.join_completion_watcher(true);
        unsafe {
            let _ = CloseHandle(self.process);
            let _ = CloseHandle(self.completion_port);
            let _ = CloseHandle(self.job);
        }
    }
}

const JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO: u32 = 4;
const WATCHER_CANCEL_MESSAGE: u32 = u32::MAX;
