use std::ffi::OsString;
use std::path::Path;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SAM_FLAGS, REG_SZ,
};

use crate::support::paths::paths_equal;

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

pub(super) fn stored_command() -> Option<String> {
    let key = open_run_key(KEY_READ).ok()?;
    let mut size = 0u32;
    let mut command = None;
    let probe = unsafe { RegQueryValueExW(key, VALUE_NAME, None, None, None, Some(&mut size)) };
    if probe == ERROR_SUCCESS && size > 0 {
        let mut buffer = vec![0u8; size as usize];
        let status = unsafe {
            RegQueryValueExW(
                key,
                VALUE_NAME,
                None,
                None,
                Some(buffer.as_mut_ptr()),
                Some(&mut size),
            )
        };
        if status == ERROR_SUCCESS {
            let units: Vec<u16> = buffer[..(size as usize).min(buffer.len())]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            command = Some(
                String::from_utf16_lossy(&units)
                    .trim_end_matches('\0')
                    .to_string(),
            );
        }
    }
    unsafe {
        let _ = RegCloseKey(key);
    }
    command
}

fn command_for(executable: &Path) -> String {
    format!("\"{}\" {}", executable.display(), TRAY_FLAG)
}

pub(super) fn stored_executable(command: &str) -> Option<&str> {
    let command = command.trim_start();
    if let Some(rest) = command.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(&rest[..end]).filter(|path| !path.is_empty());
    }
    command.split_whitespace().next()
}

pub(super) fn repaired_command(
    stored: &str,
    current: &Path,
    exists: impl Fn(&Path) -> bool,
) -> Option<String> {
    let named = Path::new(stored_executable(stored)?);
    if paths_equal(named, current) || exists(named) {
        return None;
    }
    Some(command_for(current))
}

fn write_command(command: &str) -> Result<(), String> {
    let key = open_run_key(REG_SAM_FLAGS(KEY_READ.0 | KEY_SET_VALUE.0))?;

    let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
    let status = unsafe { RegSetValueExW(key, VALUE_NAME, 0, REG_SZ, Some(bytes)) };
    unsafe {
        let _ = RegCloseKey(key);
    }
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("failed to write startup entry (code {})", status.0))
    }
}

fn delete_command() -> Result<(), String> {
    let key = open_run_key(REG_SAM_FLAGS(KEY_READ.0 | KEY_SET_VALUE.0))?;
    let status = unsafe { RegDeleteValueW(key, VALUE_NAME) };
    unsafe {
        let _ = RegCloseKey(key);
    }
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!(
            "failed to remove startup entry (code {})",
            status.0
        ))
    }
}

fn running_executable() -> Result<std::path::PathBuf, String> {
    std::env::current_exe().map_err(|error| format!("cannot resolve GameToaster path: {error}"))
}

pub(super) fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        write_command(&command_for(&running_executable()?))
    } else {
        delete_command()
    }
}

pub(super) fn repair() -> Result<(), String> {
    let Some(stored) = stored_command() else {
        return Ok(());
    };
    match repaired_command(&stored, &running_executable()?, |path| path.is_file()) {
        Some(command) => write_command(&command),
        None => Ok(()),
    }
}

pub(super) fn launched_at_sign_in(args: &[OsString]) -> bool {
    args.iter()
        .any(|argument| argument.eq_ignore_ascii_case(TRAY_FLAG))
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/platform/startup/run_key__tests.rs"]
mod tests;
