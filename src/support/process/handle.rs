use std::path::PathBuf;

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    WaitForSingleObject, PROCESS_ACCESS_RIGHTS, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

pub type CreationTime = u64;

pub const STILL_ACTIVE: u32 = 259;

pub struct ProcessHandle {
    handle: HANDLE,
    pid: u32,
}

unsafe impl Send for ProcessHandle {}

impl ProcessHandle {
    pub(super) fn open(pid: u32, rights: PROCESS_ACCESS_RIGHTS) -> Option<Self> {
        let handle = unsafe { OpenProcess(rights, false, pid) }.ok()?;
        Some(Self { handle, pid })
    }

    pub fn from_owned(handle: HANDLE, pid: u32) -> Self {
        Self { handle, pid }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn raw(&self) -> HANDLE {
        self.handle
    }

    pub fn created(&self) -> Option<CreationTime> {
        creation_time(self.handle)
    }

    pub fn has_exited(&self) -> bool {
        let wait = unsafe { WaitForSingleObject(self.handle, 0) };
        wait == WAIT_OBJECT_0
    }

    pub fn exit_code(&self) -> Option<i32> {
        if !self.has_exited() {
            return None;
        }
        let mut code = STILL_ACTIVE;
        unsafe { GetExitCodeProcess(self.handle, &mut code) }.ok()?;
        Some(code as i32)
    }

    pub fn image_path(&self) -> Option<PathBuf> {
        let mut buffer = vec![0_u16; 32_768];
        let mut length = buffer.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                self.handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
        }
        .ok()?;
        buffer.truncate(length as usize);
        Some(PathBuf::from(String::from_utf16_lossy(&buffer)))
    }

    pub fn terminate(&self, code: u32) -> bool {
        unsafe { TerminateProcess(self.handle, code) }.is_ok() || self.has_exited()
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

pub fn image_path_of(pid: u32) -> Option<PathBuf> {
    ProcessHandle::open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?.image_path()
}

pub fn creation_time_of(pid: u32) -> Option<CreationTime> {
    ProcessHandle::open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?.created()
}

pub fn creation_time(handle: HANDLE) -> Option<CreationTime> {
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) }.ok()?;
    Some(u64::from(created.dwHighDateTime) << 32 | u64::from(created.dwLowDateTime))
}
