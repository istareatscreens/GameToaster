use std::cell::Cell;
use std::mem::size_of;
use std::num::NonZeroIsize;
use std::path::PathBuf;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    Win32WindowHandle, WindowHandle,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowRect,
    GetWindowThreadProcessId, SetWindowPos, SetWindowsHookExW, UnhookWindowsHookEx, GW_OWNER,
    HCBT_ACTIVATE, HHOOK, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WH_CBT,
};

const DIALOG_CLASS: &str = "#32770";

pub fn file_dialog() -> Picker {
    let owner = foreground_owner();
    let dialog = rfd::FileDialog::new();
    Picker {
        dialog: match owner {
            Some(owner) => dialog.set_parent(&owner),
            None => dialog,
        },
        owner,
    }
}

pub struct Picker {
    dialog: rfd::FileDialog,
    owner: Option<Owner>,
}

impl Picker {
    pub fn set_title(self, title: impl Into<String>) -> Self {
        Self {
            dialog: self.dialog.set_title(title),
            ..self
        }
    }

    pub fn add_filter(self, name: impl Into<String>, extensions: &[impl ToString]) -> Self {
        Self {
            dialog: self.dialog.add_filter(name, extensions),
            ..self
        }
    }

    pub fn set_file_name(self, file_name: impl Into<String>) -> Self {
        Self {
            dialog: self.dialog.set_file_name(file_name),
            ..self
        }
    }

    pub fn pick_file(self) -> Option<PathBuf> {
        let _centred = Centred::over(self.owner);
        self.dialog.pick_file()
    }

    pub fn save_file(self) -> Option<PathBuf> {
        let _centred = Centred::over(self.owner);
        self.dialog.save_file()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Owner(NonZeroIsize);

impl HasWindowHandle for Owner {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = RawWindowHandle::Win32(Win32WindowHandle::new(self.0));

        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for Owner {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}

fn foreground_owner() -> Option<Owner> {
    let window = unsafe { GetForegroundWindow() };
    let mut window_process = 0u32;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut window_process));
    }
    owner_for(window.0 as isize, window_process, unsafe {
        GetCurrentProcessId()
    })
}

pub(crate) fn owner_for(window: isize, window_process: u32, this_process: u32) -> Option<Owner> {
    if window_process != this_process {
        return None;
    }
    NonZeroIsize::new(window).map(Owner)
}

thread_local! {

    static CENTRE_OVER: Cell<isize> = const { Cell::new(0) };
}

struct Centred {
    hook: Option<HHOOK>,
}

impl Centred {
    fn over(owner: Option<Owner>) -> Self {
        let Some(owner) = owner else {
            return Self { hook: None };
        };
        CENTRE_OVER.with(|over| over.set(owner.0.get()));
        let hook = unsafe {
            SetWindowsHookExW(WH_CBT, Some(centre_on_activate), None, GetCurrentThreadId())
        }
        .ok();
        Self { hook }
    }
}

impl Drop for Centred {
    fn drop(&mut self) {
        if let Some(hook) = self.hook.take() {
            unsafe {
                let _ = UnhookWindowsHookEx(hook);
            }
        }
        CENTRE_OVER.with(|over| over.set(0));
    }
}

unsafe extern "system" fn centre_on_activate(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HCBT_ACTIVATE as i32 {
        let window = HWND(wparam.0 as *mut _);
        let owner = CENTRE_OVER.with(Cell::get);
        if owner != 0
            && is_dialog(window)
            && GetWindow(window, GW_OWNER).is_ok_and(|found| found.0 as isize == owner)
        {
            CENTRE_OVER.with(|over| over.set(0));
            centre(window, HWND(owner as *mut _));
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn is_dialog(window: HWND) -> bool {
    let mut class = [0u16; 16];
    let length = unsafe { GetClassNameW(window, &mut class) };
    length > 0 && String::from_utf16_lossy(&class[..length as usize]) == DIALOG_CLASS
}

pub(crate) fn this_process_foreground() -> Option<isize> {
    foreground_owner().map(|owner| owner.0.get())
}

pub(crate) fn centre(window: HWND, owner: HWND) {
    let (mut dialog, mut over) = (RECT::default(), RECT::default());
    unsafe {
        if GetWindowRect(window, &mut dialog).is_err() || GetWindowRect(owner, &mut over).is_err() {
            return;
        }
    }
    let monitor = unsafe { MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return;
    }
    let (x, y) = centred_in(edges(dialog), edges(over), edges(info.rcWork));
    unsafe {
        let _ = SetWindowPos(
            window,
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

fn edges(rect: RECT) -> (i32, i32, i32, i32) {
    (rect.left, rect.top, rect.right, rect.bottom)
}

pub(crate) fn centred_in(
    dialog: (i32, i32, i32, i32),
    owner: (i32, i32, i32, i32),
    work: (i32, i32, i32, i32),
) -> (i32, i32) {
    let (width, height) = (dialog.2 - dialog.0, dialog.3 - dialog.1);
    let x = (owner.0 + owner.2) / 2 - width / 2;
    let y = (owner.1 + owner.3) / 2 - height / 2;
    (
        x.clamp(work.0, (work.2 - width).max(work.0)),
        y.clamp(work.1, (work.3 - height).max(work.1)),
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/gui/common/file_dialog__tests.rs"]
mod tests;
