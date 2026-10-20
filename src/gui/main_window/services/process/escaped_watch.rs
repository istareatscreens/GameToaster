use std::collections::BTreeSet;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    CreateEventW, OpenProcess, SetEvent, WaitForMultipleObjects, WaitForSingleObject, INFINITE,
    PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::launcher_gui::model::GameId;

const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(60);
const DISCOVERY_INTERVAL_MS: u32 = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WatchEvent {
    Attached { game_id: GameId },
    Exited { game_id: GameId },
    NotFound { game_id: GameId },
    Error { game_id: GameId, message: String },
}

pub(crate) struct EscapedProcessWatch {
    cancellation: HANDLE,
    thread: Option<JoinHandle<()>>,
}

impl EscapedProcessWatch {
    pub(crate) fn start(
        game_id: GameId,
        executable_name: String,
        executable_path: Option<PathBuf>,
        baseline: BTreeSet<u32>,
        sink: Arc<dyn Fn(WatchEvent) + Send + Sync>,
    ) -> Result<Self, String> {
        let cancellation = unsafe { CreateEventW(None, true, false, None) }.map_err(|error| {
            format!("failed to create process-watch cancellation event: {error}")
        })?;
        let raw_cancel = cancellation.0 as usize;
        let thread = match thread::Builder::new()
            .name(format!("gametoaster-watch-{}", game_id.0))
            .spawn(move || {
                let cancellation = HANDLE(raw_cancel as *mut c_void);
                let deadline = Instant::now() + DISCOVERY_TIMEOUT;
                let pid = loop {
                    if unsafe { WaitForSingleObject(cancellation, DISCOVERY_INTERVAL_MS) }
                        == WAIT_OBJECT_0
                    {
                        return;
                    }
                    let current = executable_path
                        .as_deref()
                        .map(crate::launcher_gui::process::pids_by_path)
                        .unwrap_or_else(|| {
                            crate::launcher_gui::process::pids_by_name(&executable_name)
                        });
                    let candidates = new_candidates(current, &baseline);
                    if candidates.len() == 1 {
                        break candidates[0];
                    }
                    if Instant::now() >= deadline {
                        if candidates.is_empty() {
                            sink(WatchEvent::NotFound { game_id });
                        } else {
                            sink(WatchEvent::Error {
                                game_id,
                                message: format!(
                                    "multiple new processes matched watch target '{executable_name}'"
                                ),
                            });
                        }
                        return;
                    }
                };

                let access = PROCESS_ACCESS_RIGHTS(
                    PROCESS_QUERY_LIMITED_INFORMATION.0 | SYNCHRONIZE_ACCESS,
                );
                let process = match unsafe { OpenProcess(access, false, pid) } {
                    Ok(handle) => handle,
                    Err(error) => {
                        sink(WatchEvent::Error {
                            game_id,
                            message: format!(
                                "could not attach a synchronization handle to PID {pid}: {error}"
                            ),
                        });
                        return;
                    }
                };
                sink(WatchEvent::Attached { game_id });
                let wait = unsafe {
                    WaitForMultipleObjects(&[cancellation, process], false, INFINITE)
                };
                unsafe {
                    let _ = CloseHandle(process);
                }
                if wait.0 == WAIT_OBJECT_0.0 + 1 {
                    sink(WatchEvent::Exited { game_id });
                } else if wait != WAIT_OBJECT_0 {
                    sink(WatchEvent::Error {
                        game_id,
                        message: format!("unexpected process-watch wait result 0x{:08X}", wait.0),
                    });
                }
            })
        {
            Ok(thread) => thread,
            Err(error) => {
                unsafe {
                    let _ = CloseHandle(cancellation);
                }
                return Err(format!("failed to start escaped-process watcher: {error}"));
            }
        };
        Ok(Self {
            cancellation,
            thread: Some(thread),
        })
    }

    pub(crate) fn stop(&mut self) {
        unsafe {
            let _ = SetEvent(self.cancellation);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn new_candidates(current: Vec<u32>, baseline: &BTreeSet<u32>) -> Vec<u32> {
    current
        .into_iter()
        .filter(|pid| !baseline.contains(pid))
        .collect()
}

impl Drop for EscapedProcessWatch {
    fn drop(&mut self) {
        self.stop();
        unsafe {
            let _ = CloseHandle(self.cancellation);
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/process/escaped_watch__tests.rs"]
mod tests;
