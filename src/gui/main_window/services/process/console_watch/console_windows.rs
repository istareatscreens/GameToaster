use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, SetForegroundWindow, ShowWindowAsync, GA_ROOT, SW_HIDE, SW_SHOWMINNOACTIVE,
    SW_SHOWNOACTIVATE, SW_SHOWNORMAL,
};

use super::super::window::is_console_window;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ConsoleWindow {
    pub window: isize,

    pub owner: u32,
    pub visible: bool,
    pub minimized: bool,
    pub title: String,
}

fn handle(window: isize) -> HWND {
    HWND(window as *mut _)
}

fn console(window: HWND) -> Option<ConsoleWindow> {
    if !is_console_window(window) {
        return None;
    }
    let mut owner = 0u32;
    let mut title = [0u16; 256];
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut owner));
    }
    let length = unsafe { GetWindowTextW(window, &mut title) };
    Some(ConsoleWindow {
        window: window.0 as isize,
        owner,
        visible: unsafe { IsWindowVisible(window) }.as_bool(),
        minimized: unsafe { IsIconic(window) }.as_bool(),
        title: String::from_utf16_lossy(&title[..usize::try_from(length).unwrap_or(0)]),
    })
}

pub(super) fn all() -> Vec<ConsoleWindow> {
    let mut found: Vec<ConsoleWindow> = Vec::new();
    unsafe extern "system" fn each(window: HWND, lparam: LPARAM) -> BOOL {
        let found = &mut *(lparam.0 as *mut Vec<ConsoleWindow>);
        if let Some(console) = console(window) {
            found.push(console);
        }
        TRUE
    }
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut found as *mut _ as isize));
    }
    found
}

pub(super) fn look(window: isize) -> Option<ConsoleWindow> {
    let window = handle(window);
    if !unsafe { IsWindow(window) }.as_bool() || unsafe { GetAncestor(window, GA_ROOT) } != window {
        return None;
    }
    console(window)
}

pub(super) fn exists(window: isize) -> bool {
    unsafe { IsWindow(handle(window)) }.as_bool()
}

pub(super) fn hide(window: isize) {
    unsafe {
        let _ = ShowWindowAsync(handle(window), SW_HIDE);
    }
}

pub(super) fn show(window: isize, minimized: bool) {
    let command = if minimized {
        SW_SHOWMINNOACTIVE
    } else {
        SW_SHOWNOACTIVATE
    };
    unsafe {
        let _ = ShowWindowAsync(handle(window), command);
    }
}

pub(super) fn reveal(window: isize) {
    unsafe {
        let _ = ShowWindowAsync(handle(window), SW_SHOWNORMAL);
        let _ = SetForegroundWindow(handle(window));
    }
}
