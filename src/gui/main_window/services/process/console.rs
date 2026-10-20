use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Console::{
    AttachConsole, FreeConsole, GetConsoleWindow, GetStdHandle, SetStdHandle, STD_ERROR_HANDLE,
    STD_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

pub fn hide_console_for_pid(pid: u32) -> bool {
    with_console_of(pid, || unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.0.is_null() {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    })
    .is_some()
}

pub fn with_console_of<T>(pid: u32, inside: impl FnOnce() -> T) -> Option<T> {
    if pid == 0 {
        return None;
    }
    let attached = AttachedConsole::attach(pid).ok()?;
    let result = inside();
    drop(attached);
    Some(result)
}

pub struct AttachedConsole {
    standard_handles: StandardHandles,
}

impl AttachedConsole {
    pub fn attach(pid: u32) -> windows::core::Result<Self> {
        unsafe {
            let standard_handles = StandardHandles::capture();
            AttachConsole(pid)?;
            Ok(Self { standard_handles })
        }
    }
}

impl Drop for AttachedConsole {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeConsole();
            self.standard_handles.restore();
        }
    }
}

struct StandardHandles {
    input: HANDLE,
    output: HANDLE,
    error: HANDLE,
}

impl StandardHandles {
    unsafe fn capture() -> Self {
        Self {
            input: standard_handle(STD_INPUT_HANDLE),
            output: standard_handle(STD_OUTPUT_HANDLE),
            error: standard_handle(STD_ERROR_HANDLE),
        }
    }

    unsafe fn restore(&self) {
        let _ = SetStdHandle(STD_INPUT_HANDLE, self.input);
        let _ = SetStdHandle(STD_OUTPUT_HANDLE, self.output);
        let _ = SetStdHandle(STD_ERROR_HANDLE, self.error);
    }
}

unsafe fn standard_handle(id: STD_HANDLE) -> HANDLE {
    GetStdHandle(id).unwrap_or(INVALID_HANDLE_VALUE)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/process/console__tests.rs"]
mod tests;
