use std::sync::Arc;

use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};

use super::super::framework::audit::ResourceAudit;

#[derive(Debug)]
pub(crate) struct TrackedHandle {
    handle: HANDLE,
    audit: Arc<ResourceAudit>,
}

impl TrackedHandle {
    pub fn new(handle: HANDLE, audit: &Arc<ResourceAudit>) -> Result<Self, String> {
        if handle.is_invalid() || handle == INVALID_HANDLE_VALUE {
            return Err("received an invalid Windows handle".into());
        }
        audit.handle_opened();
        Ok(Self {
            handle,
            audit: Arc::clone(audit),
        })
    }

    pub fn raw(&self) -> HANDLE {
        self.handle
    }
}

unsafe impl Send for TrackedHandle {}
unsafe impl Sync for TrackedHandle {}

impl Drop for TrackedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
        self.audit.handle_closed();
    }
}

pub(crate) struct ReaderGuard {
    audit: Arc<ResourceAudit>,
}

impl ReaderGuard {
    pub fn new(audit: &Arc<ResourceAudit>) -> Self {
        audit.reader_started();
        Self {
            audit: Arc::clone(audit),
        }
    }
}

impl Drop for ReaderGuard {
    fn drop(&mut self) {
        self.audit.reader_stopped();
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/transport/handles__tests.rs"]
mod tests;
