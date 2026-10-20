use std::fmt;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Duration;

use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Devices::Communication::{
    GetCommModemStatus, GetCommState, PurgeComm, SetCommState, SetCommTimeouts, SetupComm,
    COMMTIMEOUTS, DCB, MS_DSR_ON, NOPARITY, ONESTOPBIT, PURGE_RXABORT, PURGE_RXCLEAR,
    PURGE_TXABORT, PURGE_TXCLEAR,
};
use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_IO_PENDING, ERROR_OPERATION_ABORTED, ERROR_SHARING_VIOLATION,
    GENERIC_READ, GENERIC_WRITE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_MODE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, WaitForSingleObject,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

use super::super::framework::audit::ResourceAudit;
use super::handles::TrackedHandle;

const BAUD_RATE: u32 = 115_200;
const DATA_BITS: u8 = 8;

#[derive(Clone)]
pub(crate) struct SerialTransport {
    device: Arc<TrackedHandle>,
    cancellation: Arc<TrackedHandle>,
    io_event: Arc<TrackedHandle>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SerialOpenError {
    InUse,
    Failed(String),
}

impl fmt::Display for SerialOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InUse => formatter.write_str("the serial port is in use by another program"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl From<SerialOpenError> for String {
    fn from(error: SerialOpenError) -> Self {
        error.to_string()
    }
}

fn open_error(path: &str, error: windows::core::Error) -> SerialOpenError {
    let held = [ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION]
        .iter()
        .any(|code| error.code() == HRESULT::from_win32(code.0));
    if held {
        SerialOpenError::InUse
    } else {
        SerialOpenError::Failed(format!("failed to open serial port {path}: {error}"))
    }
}

pub(crate) fn com_port_path(name: &str) -> Option<String> {
    let name = name.trim();
    name.get(..3)
        .filter(|prefix| prefix.eq_ignore_ascii_case("COM"))
        .and_then(|_| name.get(3..))
        .and_then(|number| number.parse::<u16>().ok())
        .filter(|number| *number > 0)
        .map(|number| format!(r"\\.\COM{number}"))
}

impl SerialTransport {
    pub fn open(path: &str, audit: &Arc<ResourceAudit>) -> Result<Self, SerialOpenError> {
        let failed = |message: String| SerialOpenError::Failed(message);
        let wide = crate::support::win::wide(path);
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
        .map_err(|error| open_error(path, error))?;
        let device = Arc::new(TrackedHandle::new(device, audit).map_err(failed)?);

        let mut dcb = DCB {
            DCBlength: size_of::<DCB>() as u32,
            ..Default::default()
        };
        unsafe { GetCommState(device.raw(), &mut dcb) }
            .map_err(|error| failed(format!("{path} does not expose serial DCB state: {error}")))?;
        dcb.BaudRate = BAUD_RATE;
        dcb.ByteSize = DATA_BITS;
        dcb.Parity = NOPARITY;
        dcb.StopBits = ONESTOPBIT;
        dcb._bitfield = 1;
        unsafe { SetCommState(device.raw(), &dcb) }.map_err(|error| {
            failed(format!(
                "failed to configure {path} for 115200 8N1: {error}"
            ))
        })?;
        let timeouts = COMMTIMEOUTS {
            ReadIntervalTimeout: u32::MAX,
            ReadTotalTimeoutMultiplier: 0,
            ReadTotalTimeoutConstant: 0,
            WriteTotalTimeoutMultiplier: 0,
            WriteTotalTimeoutConstant: 500,
        };
        unsafe {
            SetCommTimeouts(device.raw(), &timeouts)
                .and_then(|_| SetupComm(device.raw(), 4096, 4096))
                .and_then(|_| {
                    PurgeComm(
                        device.raw(),
                        PURGE_RXABORT | PURGE_RXCLEAR | PURGE_TXABORT | PURGE_TXCLEAR,
                    )
                })
        }
        .map_err(|error| failed(format!("failed to initialize serial port {path}: {error}")))?;

        let cancellation = Arc::new(
            TrackedHandle::new(
                unsafe { CreateEventW(None, true, false, None) }.map_err(|error| {
                    failed(format!(
                        "failed to create serial cancellation event: {error}"
                    ))
                })?,
                audit,
            )
            .map_err(failed)?,
        );
        let io_event = Arc::new(
            TrackedHandle::new(
                unsafe { CreateEventW(None, true, false, None) }.map_err(|error| {
                    failed(format!("failed to create serial I/O event: {error}"))
                })?,
                audit,
            )
            .map_err(failed)?,
        );
        Ok(Self {
            device,
            cancellation,
            io_event,
        })
    }

    pub fn write_all(&self, bytes: &[u8], timeout: Duration) -> Result<(), String> {
        let mut offset = 0;
        while offset < bytes.len() {
            let transferred = self.overlapped_write(&bytes[offset..], timeout)?;
            if transferred == 0 {
                return Err("serial write completed without transferring data".into());
            }
            offset += transferred;
        }
        Ok(())
    }

    pub fn read_some(&self, buffer: &mut [u8], timeout: Duration) -> Result<usize, String> {
        unsafe {
            let _ = ResetEvent(self.io_event.raw());
        }
        let mut overlapped = OVERLAPPED {
            hEvent: self.io_event.raw(),
            ..Default::default()
        };
        let result =
            unsafe { ReadFile(self.device.raw(), Some(buffer), None, Some(&mut overlapped)) };
        self.complete_overlapped(result, &overlapped, timeout, "read")
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

    pub fn purge_input(&self) -> Result<(), String> {
        unsafe { PurgeComm(self.device.raw(), PURGE_RXABORT | PURGE_RXCLEAR) }
            .map_err(|error| format!("failed to purge serial input: {error}"))
    }

    pub fn sense_asserted(&self) -> Result<Option<bool>, String> {
        let mut status = Default::default();
        match unsafe { GetCommModemStatus(self.device.raw(), &mut status) } {
            Ok(()) => Ok(Some(status.0 & MS_DSR_ON.0 == 0)),
            Err(_) => Ok(None),
        }
    }

    fn overlapped_write(&self, bytes: &[u8], timeout: Duration) -> Result<usize, String> {
        unsafe {
            let _ = ResetEvent(self.io_event.raw());
        }
        let mut overlapped = OVERLAPPED {
            hEvent: self.io_event.raw(),
            ..Default::default()
        };
        let result =
            unsafe { WriteFile(self.device.raw(), Some(bytes), None, Some(&mut overlapped)) };
        self.complete_overlapped(result, &overlapped, timeout, "write")
    }

    fn complete_overlapped(
        &self,
        started: windows::core::Result<()>,
        overlapped: &OVERLAPPED,
        timeout: Duration,
        operation: &str,
    ) -> Result<usize, String> {
        if let Err(error) = started {
            if error.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
                return Err(format!("serial {operation} failed: {error}"));
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
                let _ = CancelIoEx(self.device.raw(), Some(overlapped));
            }
            return Err(format!("serial {operation} cancelled"));
        }
        if wait == WAIT_TIMEOUT {
            unsafe {
                let _ = CancelIoEx(self.device.raw(), Some(overlapped));
            }
            return Err(format!("serial {operation} timed out"));
        }
        if wait.0 != WAIT_OBJECT_0.0 + 1 {
            return Err(format!(
                "serial {operation} wait failed with 0x{:08X}",
                wait.0
            ));
        }
        let mut transferred = 0;
        unsafe { GetOverlappedResult(self.device.raw(), overlapped, &mut transferred, false) }
            .map_err(|error| {
                if error.code() == HRESULT::from_win32(ERROR_OPERATION_ABORTED.0) {
                    format!("serial {operation} cancelled")
                } else {
                    format!("serial {operation} completion failed: {error}")
                }
            })?;
        Ok(transferred as usize)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/transport/serial__tests.rs"]
mod tests;
