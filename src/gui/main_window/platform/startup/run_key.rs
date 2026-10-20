use std::ffi::OsString;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SAM_FLAGS, REG_SZ,
};

const VALUE_NAME: PCWSTR = w!("GameToaster");

const TRAY_FLAG: &str = "--tray";

fn run_key_path() -> PCWSTR {
    w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
}

fn open_run_key(access: REG_SAM_FLAGS) -> Result<HKEY, String> {
    let mut key = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            run_key_path(),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            access,
            None,
            &mut key,
            None,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(key)
    } else {
        Err(format!(
            "failed to open startup registry key (code {})",
            status.0
        ))
    }
}

pub(super) fn is_enabled() -> bool {
    let Ok(key) = open_run_key(KEY_READ) else {
        return false;
    };
    let mut size = 0u32;
    let status = unsafe { RegQueryValueExW(key, VALUE_NAME, None, None, None, Some(&mut size)) };
    unsafe {
        let _ = RegCloseKey(key);
    }
    status == ERROR_SUCCESS && size > 0
}

pub(super) fn set_enabled(enabled: bool) -> Result<(), String> {
    let command = if enabled {
        let exe = std::env::current_exe()
            .map_err(|error| format!("cannot resolve GameToaster path: {error}"))?;
        Some(format!("\"{}\" {}", exe.display(), TRAY_FLAG))
    } else {
        None
    };
    let key = open_run_key(REG_SAM_FLAGS(KEY_READ.0 | KEY_SET_VALUE.0))?;
    let result = match command {
        Some(command) => {
            let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes =
                unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
            let status = unsafe { RegSetValueExW(key, VALUE_NAME, 0, REG_SZ, Some(bytes)) };
            if status == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(format!("failed to write startup entry (code {})", status.0))
            }
        }
        None => {
            let status = unsafe { RegDeleteValueW(key, VALUE_NAME) };
            if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                Err(format!(
                    "failed to remove startup entry (code {})",
                    status.0
                ))
            }
        }
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    result
}

pub(super) fn launched_at_sign_in(args: &[OsString]) -> bool {
    args.iter()
        .any(|argument| argument.eq_ignore_ascii_case(TRAY_FLAG))
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/platform/startup/run_key__tests.rs"]
mod tests;
