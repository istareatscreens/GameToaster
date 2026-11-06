use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::*;
use crate::support::process::handle::creation_time;
use crate::support::process::tree::direct_children;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[test]
fn the_console_classes_are_the_host_and_pseudo_console_windows() {
    assert!(is_console_class("ConsoleWindowClass"));
    assert!(is_console_class("PseudoConsoleWindow"));
    for other in [
        "",
        "Window Class",
        "UnityWndClass",
        "consolewindowclass",
        "STATIC",
    ] {
        assert!(!is_console_class(other), "{other:?}");
    }
}

#[test]
fn a_programs_own_visible_console_is_not_its_window() {
    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    let mut starter = std::process::Command::new(system.join("cmd.exe"))
        .raw_arg("/d /c start \"\" /min \"%SystemRoot%\\System32\\PING.EXE\" -n 20 127.0.0.1")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .unwrap();
    let starter_pid = starter.id();
    let starter_created =
        creation_time(windows::Win32::Foundation::HANDLE(starter.as_raw_handle())).unwrap();
    let _ = starter.wait();

    let deadline = Instant::now() + Duration::from_secs(10);
    let (program, console) = loop {
        if let Some(child) = direct_children(starter_pid, starter_created)
            .into_iter()
            .next()
        {
            if let Some(window) = console_window_of(child.pid()) {
                break (child, window);
            }
        }
        assert!(Instant::now() < deadline, "the console window appeared");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(
        unsafe { IsWindowVisible(console) }.as_bool(),
        "the console is shown"
    );
    assert!(is_console_window(console));
    assert!(
        !has_visible_window(program.pid()),
        "a console alone is not a rendered window"
    );
    assert!(program.terminate(1));
}

fn console_window_of(pid: u32) -> Option<HWND> {
    struct Search {
        pid: u32,
        found: Option<HWND>,
    }
    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        let mut owner = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut owner));
        if owner == search.pid && is_console_window(hwnd) {
            search.found = Some(hwnd);
            return FALSE;
        }
        TRUE
    }
    let mut search = Search { pid, found: None };
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut search as *mut _ as isize));
    }
    search.found
}
