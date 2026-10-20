use std::ffi::c_void;
use std::sync::Arc;
use std::time::Duration;

use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Foundation::{
    ERROR_IO_PENDING, ERROR_OPERATION_ABORTED, GENERIC_READ, GENERIC_WRITE, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_MODE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, WaitForSingleObject,
};
use windows::Win32::System::IO::{CancelIoEx, DeviceIoControl, GetOverlappedResult, OVERLAPPED};

use super::super::super::framework::audit::ResourceAudit;
use super::super::super::transport::handles::TrackedHandle;

const IOCTL_MXJVS_EXCHANGE: u32 = 0x9C40_2000;

const REPLY_CAPACITY: usize = 256;

#[derive(Clone)]
pub(crate) struct MxjvsTransport {
    device: Arc<TrackedHandle>,
    cancellation: Arc<TrackedHandle>,
    io_event: Arc<TrackedHandle>,
}

impl MxjvsTransport {
    pub fn open(endpoint: &str, audit: &Arc<ResourceAudit>) -> Result<Self, String> {
        let wide = crate::support::win::wide(endpoint);

        let device = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )
        }
        .map_err(|error| format!("failed to open JVS endpoint {endpoint}: {error}"))?;
        let device = Arc::new(TrackedHandle::new(device, audit)?);
        let cancellation = Arc::new(TrackedHandle::new(
            unsafe { CreateEventW(None, true, false, None) }
                .map_err(|error| format!("failed to create mxjvs cancellation event: {error}"))?,
            audit,
        )?);
        let io_event = Arc::new(TrackedHandle::new(
            unsafe { CreateEventW(None, true, false, None) }
                .map_err(|error| format!("failed to create mxjvs I/O event: {error}"))?,
            audit,
        )?);
        Ok(Self {
            device,
            cancellation,
            io_event,
        })
    }

    pub fn exchange(&self, request: &[u8], timeout: Duration) -> Result<Vec<u8>, String> {
        unsafe {
            let _ = ResetEvent(self.io_event.raw());
        }
        let mut reply = [0u8; REPLY_CAPACITY];
        let mut returned = 0u32;
        let mut overlapped = OVERLAPPED {
            hEvent: self.io_event.raw(),
            ..Default::default()
        };
        let started = unsafe {
            DeviceIoControl(
                self.device.raw(),
                IOCTL_MXJVS_EXCHANGE,
                Some(request.as_ptr() as *const c_void),
                request.len() as u32,
                Some(reply.as_mut_ptr() as *mut c_void),
                REPLY_CAPACITY as u32,
                Some(&mut returned),
                Some(&mut overlapped),
            )
        };
        if let Err(error) = started {
            if error.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
                return Err(format!("JVS exchange failed: {error}"));
            }
        }
        let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;
        let wait = unsafe {
            WaitForMultipleObjects(
                &[self.cancellation.raw(), self.io_event.raw()],
                false,
                timeout_ms,
            )
        };
        if wait == WAIT_OBJECT_0 {
            unsafe {
                let _ = CancelIoEx(self.device.raw(), Some(&overlapped));
            }
            return Err("JVS exchange cancelled".into());
        }
        if wait == WAIT_TIMEOUT {
            unsafe {
                let _ = CancelIoEx(self.device.raw(), Some(&overlapped));
            }
            return Err("JVS exchange timed out".into());
        }
        if wait.0 != WAIT_OBJECT_0.0 + 1 {
            return Err(format!("JVS exchange wait failed with 0x{:08X}", wait.0));
        }
        unsafe { GetOverlappedResult(self.device.raw(), &overlapped, &mut returned, false) }
            .map_err(|error| {
                if error.code() == HRESULT::from_win32(ERROR_OPERATION_ABORTED.0) {
                    "JVS exchange cancelled".to_string()
                } else {
                    format!("JVS exchange completion failed: {error}")
                }
            })?;
        let length = (returned as usize).min(REPLY_CAPACITY);
        Ok(reply[..length].to_vec())
    }

    pub fn cancel(&self) {
        unsafe {
            let _ = SetEvent(self.cancellation.raw());
            let _ = CancelIoEx(self.device.raw(), None);
        }
    }

    pub fn cancelled(&self) -> bool {
        let state = unsafe { WaitForSingleObject(self.cancellation.raw(), 0) };
        state == WAIT_OBJECT_0
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/jvs/mxjvs__tests.rs"]
mod tests;
