use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects, INFINITE};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

use crate::support::win::wide;

const ERROR_IO_PENDING_CODE: i32 = HRESULT::from_win32(997).0;
const BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Default)]
struct HintState {
    ready: AtomicBool,
    dirty: AtomicBool,
    notifications: AtomicU64,
    overflows: AtomicU64,
    failures: AtomicU64,
}

impl HintState {
    fn mark_notification(&self) {
        self.notifications.fetch_add(1, Ordering::Relaxed);
        self.dirty.store(true, Ordering::Release);
    }

    fn mark_overflow(&self) {
        self.overflows.fetch_add(1, Ordering::Relaxed);
        self.dirty.store(true, Ordering::Release);
    }

    fn mark_failure(&self) {
        self.failures.fetch_add(1, Ordering::Relaxed);
        self.dirty.store(true, Ordering::Release);
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WatcherStats {
    pub notifications: u64,
    pub overflows: u64,
    pub failures: u64,
}

pub struct CatalogWatcher {
    state: Arc<HintState>,
    stop_event: usize,
    worker: Option<JoinHandle<()>>,
}

impl CatalogWatcher {
    pub fn start(catalog_root: &Path) -> Result<Self, String> {
        if !catalog_root.is_dir() {
            return Err(format!(
                "catalog watcher root is not a directory: {}",
                catalog_root.display()
            ));
        }
        let stop_event = unsafe { CreateEventW(None, true, false, None) }
            .map_err(|error| format!("failed to create catalog watcher stop event: {error}"))?;
        let state = Arc::new(HintState::default());
        let thread_state = Arc::clone(&state);
        let thread_root = catalog_root.to_path_buf();
        let stop_event_value = stop_event.0 as usize;
        let worker = match thread::Builder::new()
            .name("GameToaster-catalog-watch".into())
            .spawn(move || watch_loop(&thread_root, stop_event_value, &thread_state))
        {
            Ok(worker) => worker,
            Err(error) => {
                unsafe {
                    let _ = CloseHandle(stop_event);
                }
                return Err(format!("failed to start catalog watcher: {error}"));
            }
        };
        let watcher = Self {
            state,
            stop_event: stop_event_value,
            worker: Some(worker),
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !watcher.state.ready.load(Ordering::Acquire)
            && watcher.state.failures.load(Ordering::Acquire) == 0
            && std::time::Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        if !watcher.state.ready.load(Ordering::Acquire) {
            return Err(format!(
                "failed to arm catalog watcher for {}",
                catalog_root.display()
            ));
        }
        Ok(watcher)
    }

    pub fn take_dirty(&self) -> bool {
        self.state.dirty.swap(false, Ordering::AcqRel)
    }

    #[cfg(test)]
    pub fn stats(&self) -> WatcherStats {
        WatcherStats {
            notifications: self.state.notifications.load(Ordering::Relaxed),
            overflows: self.state.overflows.load(Ordering::Relaxed),
            failures: self.state.failures.load(Ordering::Relaxed),
        }
    }
}

impl Drop for CatalogWatcher {
    fn drop(&mut self) {
        let stop_event = HANDLE(self.stop_event as *mut std::ffi::c_void);
        unsafe {
            let _ = SetEvent(stop_event);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        unsafe {
            let _ = CloseHandle(stop_event);
        }
    }
}

fn watch_loop(root: &Path, stop_event_value: usize, state: &HintState) {
    let stop_event = HANDLE(stop_event_value as *mut std::ffi::c_void);
    let path = wide(root);
    let directory = match unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            None,
        )
    } {
        Ok(handle) => handle,
        Err(_) => {
            state.mark_failure();
            return;
        }
    };
    let change_event = match unsafe { CreateEventW(None, false, false, None) } {
        Ok(handle) => handle,
        Err(_) => {
            state.mark_failure();
            unsafe {
                let _ = CloseHandle(directory);
            }
            return;
        }
    };

    let mut buffer = Box::new([0_u8; BUFFER_SIZE]);
    loop {
        let mut overlapped = OVERLAPPED {
            hEvent: change_event,
            ..Default::default()
        };
        let queued = unsafe {
            ReadDirectoryChangesW(
                directory,
                buffer.as_mut_ptr().cast(),
                BUFFER_SIZE as u32,
                true,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE,
                None,
                Some(&mut overlapped),
                None,
            )
        };
        if let Err(error) = queued {
            if error.code().0 != ERROR_IO_PENDING_CODE {
                state.mark_failure();
                break;
            }
        }
        state.ready.store(true, Ordering::Release);

        let handles = [stop_event, change_event];
        let wait = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
        if wait == WAIT_OBJECT_0 {
            unsafe {
                let _ = CancelIoEx(directory, Some(&overlapped));
            }
            break;
        }
        if wait.0 != WAIT_OBJECT_0.0 + 1 {
            state.mark_failure();
            unsafe {
                let _ = CancelIoEx(directory, Some(&overlapped));
            }
            break;
        }

        let mut transferred = 0_u32;
        match unsafe { GetOverlappedResult(directory, &overlapped, &mut transferred, false) } {
            Ok(()) if transferred == 0 => state.mark_overflow(),
            Ok(()) => state.mark_notification(),
            Err(_) => {
                state.mark_failure();
                break;
            }
        }
    }
    unsafe {
        let _ = CloseHandle(change_event);
        let _ = CloseHandle(directory);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/catalog/watcher__tests.rs"]
mod tests;
