use windows::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindowVisible,
};

pub const CONSOLE_WINDOW_CLASSES: [&str; 2] = ["ConsoleWindowClass", "PseudoConsoleWindow"];

pub fn is_console_class(class: &str) -> bool {
    CONSOLE_WINDOW_CLASSES.contains(&class)
}

pub fn class_name(window: HWND) -> String {
    let mut name = [0u16; 256];
    let length = unsafe { GetClassNameW(window, &mut name) };
    String::from_utf16_lossy(&name[..usize::try_from(length).unwrap_or(0)])
}

pub fn is_console_window(window: HWND) -> bool {
    is_console_class(&class_name(window))
}

struct VisibleProbe {
    pid: u32,
    found: bool,
}

pub fn has_visible_window(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let mut probe = VisibleProbe { pid, found: false };
    unsafe {
        let _ = EnumWindows(
            Some(visible_enum_proc),
            LPARAM(&mut probe as *mut _ as isize),
        );
    }
    probe.found
}

unsafe extern "system" fn visible_enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let probe = &mut *(lparam.0 as *mut VisibleProbe);
    let mut wpid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut wpid));
    if wpid == probe.pid && IsWindowVisible(hwnd).as_bool() && !is_console_window(hwnd) {
        probe.found = true;
        return FALSE;
    }
    TRUE
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/process/window__tests.rs"]
mod tests;
