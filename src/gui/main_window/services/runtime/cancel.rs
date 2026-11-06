use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{CreateEventW, SetEvent};

use crate::plugin::runtime::OperationCancel;

#[derive(Clone)]
pub struct LaunchCancel {
    inner: Arc<Inner>,
}

struct Inner {
    cancelled: AtomicBool,

    event: usize,

    operation: Mutex<Option<OperationCancel>>,
}

impl LaunchCancel {
    pub fn new() -> Self {
        let event = unsafe { CreateEventW(None, true, false, None) }
            .map(|event| event.0 as usize)
            .unwrap_or_default();
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                event,
                operation: Mutex::new(None),
            }),
        }
    }

    pub fn cancel(&self) {
        if self.inner.cancelled.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Some(event) = self.event() {
            unsafe {
                let _ = SetEvent(event);
            }
        }
        if let Some(operation) = self.lock().as_ref() {
            let _ = operation.cancel();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    pub fn event(&self) -> Option<HANDLE> {
        (self.inner.event != 0).then(|| HANDLE(self.inner.event as *mut _))
    }

    pub fn set_operation(&self, operation: Option<OperationCancel>) {
        let mut current = self.lock();
        *current = operation;
        if self.is_cancelled() {
            if let Some(operation) = current.as_ref() {
                let _ = operation.cancel();
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<OperationCancel>> {
        self.inner
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for LaunchCancel {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for LaunchCancel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LaunchCancel")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if self.event != 0 {
            unsafe {
                let _ = CloseHandle(HANDLE(self.event as *mut _));
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/runtime/cancel__tests.rs"]
mod tests;
