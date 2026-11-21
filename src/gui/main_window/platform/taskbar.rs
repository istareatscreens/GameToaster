use std::num::NonZeroIsize;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

pub fn window_of(window: &impl HasWindowHandle) -> Option<NonZeroIsize> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd),
        _ => None,
    }
}

pub(crate) fn without_taskbar_button(style: isize) -> isize {
    (style | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize)
}

pub(crate) fn with_taskbar_button(style: isize) -> isize {
    (style | WS_EX_APPWINDOW.0 as isize) & !(WS_EX_TOOLWINDOW.0 as isize)
}

pub fn show_next_without_taskbar_button(window: NonZeroIsize) -> bool {
    set_extended_style(window, without_taskbar_button)
}

pub fn show_next_with_taskbar_button(window: NonZeroIsize) -> bool {
    set_extended_style(window, with_taskbar_button)
}

fn set_extended_style(window: NonZeroIsize, change: fn(isize) -> isize) -> bool {
    let hwnd = HWND(window.get() as *mut core::ffi::c_void);
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let wanted = change(style);
    if wanted == style {
        return true;
    }

    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
    }
    let written = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    written == wanted
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/platform/taskbar__tests.rs"]
mod tests;
