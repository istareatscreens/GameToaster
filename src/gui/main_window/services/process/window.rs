use windows::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
};

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
    if wpid == probe.pid && IsWindowVisible(hwnd).as_bool() {
        probe.found = true;
        return FALSE;
    }
    TRUE
}
