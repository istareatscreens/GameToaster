use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects, INFINITE};

use super::channel::{ChannelListener, ChannelServer, Invalid, Message, ResultMessage};
use super::process::{PluginProcess, ProgramState, StartSpec};
use super::request::{new_operation_id, Operation, RequestContext, RequestFile};
use crate::support::process::console_policy::ConsoleSettings;
use crate::support::process::handle::CreationTime;

#[derive(Clone)]
pub struct OperationSpec {
    pub executable: PathBuf,
    pub working_directory: PathBuf,
    pub session_state_root: PathBuf,
    pub operation: Operation,
    pub request_context: RequestContext,
    pub timeout: Duration,
    pub environment: Vec<(OsString, OsString)>,

    pub console: ConsoleSettings,
    pub request_repaint: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildResult {
    pub code: i32,
    pub result: Option<ResultMessage>,
    pub output: Option<String>,
}

impl ChildResult {
    pub fn succeeded(&self) -> bool {
        self.code == 0
    }

    pub fn reason(&self) -> Option<String> {
        self.result
            .as_ref()
            .map(|result| result.text.clone())
            .filter(|text| !text.is_empty())
            .or_else(|| self.output.clone())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeEvent {
    Started {
        process_id: u32,
        created: CreationTime,
    },
    Connected {
        client_pid: u32,
    },
    Message(Message),
    Invalid(Invalid),

    Ignored {
        message_type: String,
    },
    Exited(ChildResult),
    TimedOut,
    LauncherCancelled,
    WorkerFailure(String),
}

pub struct OperationHandle {
    receiver: Receiver<RuntimeEvent>,
    cancel: OperationCancel,
    worker: Option<JoinHandle<()>>,
    pub process_id: u32,
    pub created: CreationTime,

    pub operation_id: String,
}

#[derive(Clone)]
pub struct OperationCancel {
    inner: Arc<CancelEvent>,
}

struct CancelEvent {
    handle: usize,
}

impl OperationCancel {
    pub fn cancel(&self) -> Result<(), String> {
        unsafe { SetEvent(handle(self.inner.handle)) }
            .map_err(|error| format!("failed to signal plugin cancellation: {error}"))
    }
}

impl Drop for CancelEvent {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(handle(self.handle));
        }
    }
}

impl OperationHandle {
    pub fn recv(&self) -> Result<RuntimeEvent, String> {
        self.receiver
            .recv()
            .map_err(|_| "runtime worker event channel closed".to_string())
    }

    pub fn cancel_token(&self) -> OperationCancel {
        self.cancel.clone()
    }

    pub fn join(mut self) -> Result<(), String> {
        self.join_inner()
    }

    fn join_inner(&mut self) -> Result<(), String> {
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| "runtime worker panicked".to_string())?;
        }
        Ok(())
    }
}

impl Drop for OperationHandle {
    fn drop(&mut self) {
        if self.worker.is_some() {
            let _ = self.cancel.cancel();
            let _ = self.join_inner();
        }
    }
}

struct Forwarder {
    sender: Sender<RuntimeEvent>,
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
    last_result: Mutex<Option<ResultMessage>>,
}

impl Forwarder {
    fn send(&self, event: RuntimeEvent) {
        let _ = self.sender.send(event);
        if let Some(repaint) = &self.repaint {
            repaint();
        }
    }

    fn last_result(&self) -> Option<ResultMessage> {
        self.last_result
            .lock()
            .ok()
            .and_then(|result| result.clone())
    }
}

impl ChannelListener for Forwarder {
    fn connected(&self, client_pid: u32) {
        self.send(RuntimeEvent::Connected { client_pid });
    }

    fn message(&self, message: Message) {
        if let Message::Result(result) = &message {
            if let Ok(mut last) = self.last_result.lock() {
                *last = Some(result.clone());
            }
        }
        self.send(RuntimeEvent::Message(message));
    }

    fn invalid(&self, reason: Invalid) {
        self.send(RuntimeEvent::Invalid(reason));
    }

    fn ignored(&self, message_type: String) {
        self.send(RuntimeEvent::Ignored { message_type });
    }

    fn disconnected(&self, _client_pid: u32) {}
}

pub struct RuntimeHost;

struct WorkerInput {
    process: PluginProcess,
    server: ChannelServer,
    forwarder: Arc<Forwarder>,
    request_file: RequestFile,
    cancel_event: usize,
    timeout: Duration,
}

impl RuntimeHost {
    pub fn start(spec: OperationSpec) -> Result<OperationHandle, String> {
        validate_spec(&spec)?;
        let (sender, receiver) = mpsc::channel();
        let forwarder = Arc::new(Forwarder {
            sender: sender.clone(),
            repaint: spec.request_repaint,
            last_result: Mutex::new(None),
        });
        let operation_id = new_operation_id()?;
        let server = ChannelServer::start(&operation_id, forwarder.clone())?;
        let request_file = RequestFile::create(
            &spec.session_state_root,
            spec.operation,
            &spec.request_context,
            &operation_id,
            server.name(),
        )?;
        let arguments = request_file.argv();
        let mut environment = spec.environment.clone();
        environment.extend(request_file.environment());

        let cancel_event = unsafe { CreateEventW(None, true, false, None) }
            .map_err(|error| format!("failed to create operation cancellation event: {error}"))?;
        let cancel_value = cancel_event.0 as usize;
        let cancel = OperationCancel {
            inner: Arc::new(CancelEvent {
                handle: cancel_value,
            }),
        };
        let process = PluginProcess::start(
            StartSpec {
                executable: &spec.executable,
                arguments: &arguments,
                working_directory: &spec.working_directory,
                environment: &environment,
                console: spec.console,
            },
            || request_file.capture_files(),
        )
        .map_err(|error| {
            format!(
                "failed to start plugin executable {} for {}: {error}",
                spec.executable.display(),
                spec.operation.as_str()
            )
        })?;
        let process_id = process.id();
        let created = process.created();
        sender
            .send(RuntimeEvent::Started {
                process_id,
                created,
            })
            .map_err(|_| "failed to initialize runtime worker channel".to_string())?;
        let input = Arc::new(Mutex::new(Some(WorkerInput {
            process,
            server,
            forwarder,
            request_file,
            cancel_event: cancel_value,
            timeout: spec.timeout,
        })));
        let worker_input = Arc::clone(&input);
        let worker = match thread::Builder::new()
            .name("GameToaster-plugin-runtime".into())
            .spawn(move || {
                let input = worker_input.lock().ok().and_then(|mut input| input.take());
                if let Some(input) = input {
                    worker_loop(input, sender);
                }
            }) {
            Ok(worker) => worker,
            Err(error) => {
                if let Ok(mut input) = input.lock() {
                    if let Some(mut input) = input.take() {
                        input.process.kill();
                    }
                }
                unsafe {
                    let _ = SetEvent(cancel_event);
                }
                return Err(format!("failed to start plugin runtime worker: {error}"));
            }
        };
        Ok(OperationHandle {
            receiver,
            cancel,
            worker: Some(worker),
            process_id,
            created,
            operation_id,
        })
    }
}

const LOOK_AGAIN_MS: u32 = 250;

fn worker_loop(input: WorkerInput, sender: Sender<RuntimeEvent>) {
    let WorkerInput {
        mut process,
        mut server,
        forwarder,
        request_file,
        cancel_event,
        timeout,
    } = input;
    let handles = [process.handle(), handle(cancel_event)];
    let wait_ms = timeout.as_millis().min(u128::from(u32::MAX - 1)) as u32;
    let mut wait = unsafe { WaitForMultipleObjects(&handles, false, wait_ms) };
    while wait == WAIT_TIMEOUT {
        wait = match process.program_state() {
            ProgramState::Running => break,
            ProgramState::Ended => unsafe { WaitForMultipleObjects(&handles, false, INFINITE) },
            ProgramState::Unseen => unsafe {
                WaitForMultipleObjects(&handles, false, LOOK_AGAIN_MS)
            },
        };
    }
    let event = if wait == WAIT_OBJECT_0 {
        match process.exit_code() {
            Ok(code) => {
                server.shutdown();
                RuntimeEvent::Exited(ChildResult {
                    code,
                    result: forwarder.last_result(),
                    output: process
                        .captures_output()
                        .then(|| read_child_output(&request_file))
                        .flatten(),
                })
            }
            Err(error) => {
                server.shutdown();
                RuntimeEvent::WorkerFailure(format!(
                    "failed to collect plugin exit status: {error}"
                ))
            }
        }
    } else {
        process.kill();
        server.shutdown();
        if wait.0 == WAIT_OBJECT_0.0 + 1 {
            RuntimeEvent::LauncherCancelled
        } else if wait == WAIT_TIMEOUT {
            RuntimeEvent::TimedOut
        } else if wait == WAIT_FAILED {
            RuntimeEvent::WorkerFailure("WaitForMultipleObjects failed for plugin operation".into())
        } else {
            RuntimeEvent::WorkerFailure(format!("unexpected plugin wait result {}", wait.0))
        }
    };
    let _ = sender.send(event);
}

const OUTPUT_TAIL_LIMIT: u64 = 16 * 1024;

fn read_child_output(request_file: &RequestFile) -> Option<String> {
    let (stdout_path, stderr_path) = request_file.output_paths();
    let stderr = read_output_tail(stderr_path, OUTPUT_TAIL_LIMIT).unwrap_or_default();
    let remaining = OUTPUT_TAIL_LIMIT.saturating_sub(stderr.len() as u64);
    let stdout = if remaining == 0 {
        String::new()
    } else {
        read_output_tail(stdout_path, remaining).unwrap_or_default()
    };
    match (stderr.is_empty(), stdout.is_empty()) {
        (false, false) => Some(format!("{stderr}\n{stdout}")),
        (false, true) => Some(stderr),
        (true, false) => Some(stdout),
        (true, true) => None,
    }
}

fn read_output_tail(path: &Path, limit: u64) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("failed to read plugin output {}: {error}", path.display()))?;
    let length = file
        .metadata()
        .map_err(|error| {
            format!(
                "failed to inspect plugin output {}: {error}",
                path.display()
            )
        })?
        .len();
    if length > limit {
        file.seek(SeekFrom::End(-(limit as i64)))
            .map_err(|error| format!("failed to seek plugin output {}: {error}", path.display()))?;
    }
    let mut bytes = Vec::with_capacity(length.min(limit) as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read plugin output {}: {error}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes).replace('\0', "");
    Ok(text.trim().to_string())
}

fn validate_spec(spec: &OperationSpec) -> Result<(), String> {
    if !spec.executable.is_absolute() || !spec.executable.is_file() {
        return Err(format!(
            "plugin executable is missing or not absolute: {}",
            spec.executable.display()
        ));
    }
    if !spec.working_directory.is_absolute() || !spec.working_directory.is_dir() {
        return Err(format!(
            "plugin working directory is missing or not absolute: {}",
            spec.working_directory.display()
        ));
    }
    if !spec.session_state_root.is_absolute() {
        return Err("plugin session state root must be absolute".into());
    }
    if spec.timeout.is_zero() {
        return Err("plugin operation timeout must be positive".into());
    }
    Ok(())
}

fn handle(value: usize) -> HANDLE {
    HANDLE(value as *mut std::ffi::c_void)
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/runtime/host__tests.rs"]
pub(crate) mod tests;
