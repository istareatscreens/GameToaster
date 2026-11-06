use std::collections::HashSet;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, TRUE};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
};

use super::process::is_console_window;
use crate::gui::common::file_dialog;
use crate::support::process::handle::{creation_time_of, CreationTime};
use crate::support::process::tree::descendants;

pub const MAIN_WINDOW_TITLE: &str = "GameToaster";

pub const WINDOW_LIMIT: Duration = Duration::from_secs(15);

const LOOK: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub process: u32,
    pub visible: bool,
    pub owned: bool,
    pub console: bool,
}

pub fn is_target(window: Candidate, processes: &HashSet<u32>) -> bool {
    window.visible && !window.owned && !window.console && processes.contains(&window.process)
}

pub fn place_over_main_window(process: u32) {
    let Some(owner) = main_window().or_else(file_dialog::this_process_foreground) else {
        return;
    };

    let Some(created) = creation_time_of(process) else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("gametoaster-config-placement".into())
        .spawn(move || place(process, created, owner));
}

fn place(process: u32, created: CreationTime, owner: isize) {
    let deadline = Instant::now() + WINDOW_LIMIT;
    while Instant::now() < deadline {
        let mut processes: HashSet<u32> = descendants(process, created)
            .iter()
            .map(|handle| handle.pid())
            .collect();
        processes.insert(process);
        if let Some(window) = first_target(&processes) {
            file_dialog::centre(window, HWND(owner as *mut _));
            return;
        }
        std::thread::sleep(LOOK);
    }
}

fn main_window() -> Option<isize> {
    let mut found: Option<isize> = None;
    unsafe {
        let _ = EnumWindows(Some(visit_own), LPARAM(&mut found as *mut _ as isize));
    }
    found
}

unsafe extern "system" fn visit_own(window: HWND, data: LPARAM) -> BOOL {
    let found = &mut *(data.0 as *mut Option<isize>);
    let mut process = 0u32;
    GetWindowThreadProcessId(window, Some(&mut process));
    if process != GetCurrentProcessId()
        || !IsWindowVisible(window).as_bool()
        || GetWindow(window, GW_OWNER).is_ok_and(|owner| !owner.is_invalid())
    {
        return TRUE;
    }
    let mut title = [0u16; 64];
    let length = GetWindowTextW(window, &mut title);
    if length > 0 && is_main_title(&String::from_utf16_lossy(&title[..length as usize])) {
        *found = Some(window.0 as isize);
        return FALSE;
    }
    TRUE
}

pub fn is_main_title(title: &str) -> bool {
    title == MAIN_WINDOW_TITLE
}

struct Search<'a> {
    processes: &'a HashSet<u32>,
    found: Option<HWND>,
}

fn first_target(processes: &HashSet<u32>) -> Option<HWND> {
    let mut search = Search {
        processes,
        found: None,
    };
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut search as *mut _ as isize));
    }
    search.found
}

unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> BOOL {
    let search = &mut *(data.0 as *mut Search);
    let mut process = 0u32;
    GetWindowThreadProcessId(window, Some(&mut process));
    let candidate = Candidate {
        process,
        visible: IsWindowVisible(window).as_bool(),
        owned: GetWindow(window, GW_OWNER).is_ok_and(|owner| !owner.is_invalid()),
        console: is_console_window(window),
    };
    if is_target(candidate, search.processes) {
        search.found = Some(window);
        return FALSE;
    }
    TRUE
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/services/config_placement__tests.rs"]
mod tests;
