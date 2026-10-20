use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_IO_PENDING, ERROR_NO_DATA,
    ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE, WAIT_FAILED, WAIT_OBJECT_0,
};
use windows::Win32::Storage::FileSystem::{
    ReadFile, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_INBOUND,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
    PeekNamedPipe, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, WaitForSingleObject, INFINITE,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

use super::framing::{Frame, LineSplitter};
use super::messages::{message_type, parse_line, Invalid, Message, MAX_LINE_BYTES};
use super::security::SecurityDescriptor;
use crate::support::win::wide;

const MAX_CONNECTIONS: usize = 8;

const INVALID_LINE_BUDGET: u32 = 1_000;
const READ_CHUNK: usize = 4_096;

pub fn pipe_name(operation_id: &str) -> String {
    format!(r"\\.\pipe\GameToaster.v2.{operation_id}")
}

pub trait ChannelListener: Send + Sync {
    fn connected(&self, client_pid: u32);
    fn message(&self, message: Message);

    fn invalid(&self, reason: Invalid);

    fn ignored(&self, message_type: String);
    fn disconnected(&self, client_pid: u32);
}

pub struct ChannelServer {
    name: String,
    stop: Arc<Event>,
    acceptor: Option<JoinHandle<()>>,
    readers: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl ChannelServer {
    pub fn start(operation_id: &str, listener: Arc<dyn ChannelListener>) -> Result<Self, String> {
        let name = pipe_name(operation_id);
        let descriptor = SecurityDescriptor::current_user_only()?;
        let first = create_instance(&name, &descriptor, true)?;
        let stop = Arc::new(Event::manual_reset()?);
        let readers = Arc::new(Mutex::new(Vec::new()));
        let acceptor = thread::Builder::new()
            .name("GameToaster-plugin-channel".into())
            .spawn({
                let name = name.clone();
                let stop = Arc::clone(&stop);
                let readers = Arc::clone(&readers);
                move || accept_loop(name, descriptor, first, stop, listener, readers)
            })
            .map_err(|error| format!("failed to start the channel acceptor: {error}"))?;
        Ok(Self {
            name,
            stop,
            acceptor: Some(acceptor),
            readers,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn shutdown(&mut self) {
        self.stop.set();
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.join();
        }
        let readers = match self.readers.lock() {
            Ok(mut readers) => std::mem::take(&mut *readers),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };
        for reader in readers {
            let _ = reader.join();
        }
    }
}

impl Drop for ChannelServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl std::fmt::Debug for ChannelServer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChannelServer")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

fn accept_loop(
    name: String,
    descriptor: SecurityDescriptor,
    first: Instance,
    stop: Arc<Event>,
    listener: Arc<dyn ChannelListener>,
    readers: Arc<Mutex<Vec<JoinHandle<()>>>>,
) {
    let mut listening = Vec::with_capacity(LISTENING_INSTANCES);
    match Listening::post(first) {
        Ok(first) => listening.push(first),
        Err(_) => return,
    }
    let mut accepted = 0;
    while accepted < MAX_CONNECTIONS {
        while listening.len() < LISTENING_INSTANCES {
            match create_instance(&name, &descriptor, false).and_then(Listening::post) {
                Ok(instance) => listening.push(instance),
                Err(_) => break,
            }
        }
        if listening.is_empty() {
            break;
        }
        let mut handles = vec![stop.handle()];
        handles.extend(listening.iter().map(|instance| instance.event.handle()));
        let wait = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
        let index = wait.0.wrapping_sub(WAIT_OBJECT_0.0) as usize;
        if wait == WAIT_FAILED || index > listening.len() {
            break;
        }
        if index == 0 {
            for instance in listening.drain(..) {
                if instance.event.is_set() {
                    spawn_reader(instance.connected(), &stop, &listener, &readers);
                }
            }
            break;
        }
        let connected = listening.remove(index - 1).connected();
        accepted += 1;
        if !spawn_reader(connected, &stop, &listener, &readers) {
            break;
        }
    }
}

fn spawn_reader(
    connected: Instance,
    stop: &Arc<Event>,
    listener: &Arc<dyn ChannelListener>,
    readers: &Arc<Mutex<Vec<JoinHandle<()>>>>,
) -> bool {
    let reader = thread::Builder::new()
        .name("GameToaster-plugin-channel-reader".into())
        .spawn({
            let stop = Arc::clone(stop);
            let listener = Arc::clone(listener);
            move || read_loop(connected, stop, listener)
        });
    match reader {
        Ok(reader) => {
            if let Ok(mut readers) = readers.lock() {
                readers.push(reader);
            }
            true
        }
        Err(_) => false,
    }
}

const LISTENING_INSTANCES: usize = 2;

struct Listening {
    instance: Option<Instance>,
    event: Event,
    overlapped: Box<OVERLAPPED>,
}

impl Listening {
    fn post(instance: Instance) -> Result<Self, String> {
        let event = Event::manual_reset()?;
        let mut overlapped = Box::new(OVERLAPPED {
            hEvent: event.handle(),
            ..Default::default()
        });
        let started = unsafe { ConnectNamedPipe(instance.handle(), Some(&mut *overlapped)) };
        match started {
            Ok(()) => event.set(),
            Err(error)
                if [ERROR_PIPE_CONNECTED.0, ERROR_NO_DATA.0].contains(&win32_code(&error)) =>
            {
                event.set()
            }
            Err(error) if win32_code(&error) == ERROR_IO_PENDING.0 => {}
            Err(error) => return Err(format!("failed to listen on the channel: {error}")),
        }
        Ok(Self {
            instance: Some(instance),
            event,
            overlapped,
        })
    }

    fn connected(mut self) -> Instance {
        self.instance.take().expect("a listening instance")
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        if let Some(instance) = &self.instance {
            let _ = unsafe { CancelIoEx(instance.handle(), Some(&*self.overlapped)) };
            let mut transferred = 0_u32;
            let _ = unsafe {
                GetOverlappedResult(instance.handle(), &*self.overlapped, &mut transferred, true)
            };
        }
    }
}

fn read_loop(instance: Instance, stop: Arc<Event>, listener: Arc<dyn ChannelListener>) {
    let client_pid = client_pid(&instance);
    listener.connected(client_pid);
    if let Ok(event) = Event::manual_reset() {
        let mut splitter = LineSplitter::new(MAX_LINE_BYTES);
        let mut invalid_lines = 0_u32;
        let mut unknown_types = 0_u32;
        let mut buffer = vec![0_u8; READ_CHUNK];

        let mut left_after_stop: Option<usize> = None;
        'reading: loop {
            if left_after_stop.is_none() && stop.is_set() {
                left_after_stop = Some(buffered_bytes(&instance));
            }
            let outcome = match left_after_stop {
                None => read_chunk(&instance, &event, &stop, &mut buffer),
                Some(0) => break,
                Some(left) => {
                    let chunk = left.min(buffer.len());
                    read_buffered(&instance, &event, &mut buffer[..chunk])
                }
            };
            let read = match outcome {
                Read::Bytes(read) => read,
                Read::Stopped => continue,
                Read::Ended => break,
            };
            if let Some(left) = left_after_stop.as_mut() {
                *left = left.saturating_sub(read);
            }
            for frame in splitter.push(&buffer[..read]) {
                let (line, outcome) = match frame {
                    Frame::Line(line) => {
                        let outcome = parse_line(&line);
                        (line, outcome)
                    }
                    Frame::Overlong => (Vec::new(), Err(Invalid::Overlong)),
                };
                match outcome {
                    Ok(Some(message)) => listener.message(message),
                    Ok(None) => {
                        unknown_types += 1;
                        if unknown_types == 1 {
                            listener.ignored(message_type(&line));
                        }
                    }
                    Err(reason) => {
                        invalid_lines += 1;
                        if invalid_lines == 1 {
                            listener.invalid(reason);
                        }
                        if invalid_lines >= INVALID_LINE_BUDGET {
                            break 'reading;
                        }
                    }
                }
            }
        }
    }
    drop(instance);
    listener.disconnected(client_pid);
}

enum Read {
    Bytes(usize),

    Stopped,

    Ended,
}

fn read_chunk(instance: &Instance, event: &Event, stop: &Event, buffer: &mut [u8]) -> Read {
    let _ = unsafe { ResetEvent(event.handle()) };
    let mut overlapped = OVERLAPPED {
        hEvent: event.handle(),
        ..Default::default()
    };
    let started = unsafe { ReadFile(instance.handle(), Some(buffer), None, Some(&mut overlapped)) };
    if let Err(error) = started {
        if win32_code(&error) != ERROR_IO_PENDING.0 {
            return Read::Ended;
        }
        if wait_signalled(event, stop) == Signalled::Stop {
            let _ = unsafe { CancelIoEx(instance.handle(), Some(&overlapped)) };
            let mut transferred = 0_u32;

            return match unsafe {
                GetOverlappedResult(instance.handle(), &overlapped, &mut transferred, true)
            } {
                Ok(()) if transferred > 0 => Read::Bytes(transferred as usize),
                _ => Read::Stopped,
            };
        }
    }
    let mut read = 0_u32;
    match unsafe { GetOverlappedResult(instance.handle(), &overlapped, &mut read, false) } {
        Ok(()) => Read::Bytes(read as usize),
        Err(_) => Read::Ended,
    }
}

fn read_buffered(instance: &Instance, event: &Event, buffer: &mut [u8]) -> Read {
    let _ = unsafe { ResetEvent(event.handle()) };
    let mut overlapped = OVERLAPPED {
        hEvent: event.handle(),
        ..Default::default()
    };
    let started = unsafe { ReadFile(instance.handle(), Some(buffer), None, Some(&mut overlapped)) };
    if let Err(error) = started {
        if win32_code(&error) != ERROR_IO_PENDING.0 {
            return Read::Ended;
        }
    }
    let mut read = 0_u32;
    match unsafe { GetOverlappedResult(instance.handle(), &overlapped, &mut read, true) } {
        Ok(()) => Read::Bytes(read as usize),
        Err(_) => Read::Ended,
    }
}

fn buffered_bytes(instance: &Instance) -> usize {
    let mut available = 0_u32;
    match unsafe { PeekNamedPipe(instance.handle(), None, 0, None, Some(&mut available), None) } {
        Ok(()) => available as usize,
        Err(_) => 0,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Signalled {
    Io,
    Stop,
}

fn wait_signalled(io: &Event, stop: &Event) -> Signalled {
    let wait = unsafe { WaitForMultipleObjects(&[io.handle(), stop.handle()], false, INFINITE) };
    if wait == WAIT_OBJECT_0 {
        Signalled::Io
    } else {
        Signalled::Stop
    }
}

fn client_pid(instance: &Instance) -> u32 {
    let mut pid = 0_u32;
    let _ = unsafe { GetNamedPipeClientProcessId(instance.handle(), &mut pid) };
    pid
}

fn create_instance(
    name: &str,
    descriptor: &SecurityDescriptor,
    first: bool,
) -> Result<Instance, String> {
    let wide_name = wide(name);
    let mut open_mode = PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED;
    if first {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    let attributes = descriptor.attributes();
    let handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(wide_name.as_ptr()),
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            MAX_CONNECTIONS as u32,
            0,
            MAX_LINE_BYTES as u32,
            0,
            Some(&attributes),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let error = unsafe { GetLastError() };
        return Err(if first && error == ERROR_ACCESS_DENIED {
            format!("channel name is already in use: {name}")
        } else {
            format!("failed to create the channel pipe {name}: {error:?}")
        });
    }
    Ok(Instance(handle.0 as usize))
}

fn win32_code(error: &windows::core::Error) -> u32 {
    error.code().0 as u32 & 0xffff
}

struct Instance(usize);

impl Instance {
    fn handle(&self) -> HANDLE {
        HANDLE(self.0 as *mut std::ffi::c_void)
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            let _ = DisconnectNamedPipe(self.handle());
            let _ = CloseHandle(self.handle());
        }
    }
}

struct Event(usize);

impl Event {
    fn manual_reset() -> Result<Self, String> {
        let handle = unsafe { CreateEventW(None, true, false, None) }
            .map_err(|error| format!("failed to create a channel event: {error}"))?;
        Ok(Self(handle.0 as usize))
    }

    fn handle(&self) -> HANDLE {
        HANDLE(self.0 as *mut std::ffi::c_void)
    }

    fn set(&self) {
        let _ = unsafe { SetEvent(self.handle()) };
    }

    fn is_set(&self) -> bool {
        unsafe { WaitForSingleObject(self.handle(), 0) == WAIT_OBJECT_0 }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle());
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/plugin/runtime/channel/server__tests.rs"]
mod tests;
