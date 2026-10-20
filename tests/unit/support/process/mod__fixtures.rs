use std::time::Duration;

use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::System::Threading::{
    WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
};

use super::handle::ProcessHandle;

pub(crate) fn hold(pid: u32) -> ProcessHandle {
    ProcessHandle::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE)
        .unwrap_or_else(|| panic!("process {pid} can be opened"))
}

pub(crate) fn ends_within(process: &ProcessHandle, timeout: Duration) -> bool {
    let wait_ms = timeout.as_millis().min(u128::from(u32::MAX - 1)) as u32;
    unsafe { WaitForSingleObject(process.raw(), wait_ms) == WAIT_OBJECT_0 }
}
