use std::path::Path;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};

use crate::support::process::handle::{image_path_of, STILL_ACTIVE};
use crate::support::process::snapshot::{snapshot, SnapshotEntry};

pub fn process_name_matches(actual: &str, target: &str) -> bool {
    let normalize = |name: &str| {
        name.trim()
            .to_ascii_lowercase()
            .trim_end_matches(".exe")
            .to_string()
    };
    let target = normalize(target);
    !target.is_empty() && normalize(actual) == target
}

pub fn pids_by_name(name: &str) -> Vec<u32> {
    if name.trim().is_empty() {
        return Vec::new();
    }
    snapshot()
        .into_iter()
        .filter(|entry| process_name_matches(&entry.name, name))
        .map(|entry| entry.pid)
        .collect()
}

pub fn is_process_running(name: &str) -> bool {
    !pids_by_name(name).is_empty()
}

pub fn find_pid_by_name(name: &str) -> Option<u32> {
    pids_by_name(name).into_iter().next()
}

pub fn process_path_matches(actual: &Path, target: &Path) -> bool {
    fn normalized(path: &Path) -> String {
        path.as_os_str()
            .to_string_lossy()
            .strip_prefix(r"\\?\")
            .unwrap_or(&path.as_os_str().to_string_lossy())
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    }
    let target = normalized(target);
    !target.is_empty() && normalized(actual) == target
}

pub fn pids_by_path(path: &Path) -> Vec<u32> {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    pids_by_name(name)
        .into_iter()
        .filter(|pid| {
            image_path_of(*pid)
                .as_deref()
                .is_some_and(|actual| process_path_matches(actual, path))
        })
        .collect()
}

pub fn find_pid_by_path(path: &Path) -> Option<u32> {
    pids_by_path(path).into_iter().next()
}

pub fn is_process_path_running(path: &Path) -> bool {
    find_pid_by_path(path).is_some()
}

pub struct ProcessWatch {
    handle: HANDLE,
}

impl ProcessWatch {
    pub fn open(pid: u32) -> Option<Self> {
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        Some(Self { handle })
    }

    pub fn exit_code(&self) -> Option<u32> {
        let mut code = 0u32;
        let ok = unsafe { GetExitCodeProcess(self.handle, &mut code) }.is_ok();
        if ok && code == STILL_ACTIVE {
            None
        } else {
            Some(code)
        }
    }
}

impl Drop for ProcessWatch {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

pub fn terminate_by_name(name: &str) -> usize {
    if name.trim().is_empty() {
        return 0;
    }
    let table = snapshot();
    let targets = table
        .iter()
        .filter(|entry| process_name_matches(&entry.name, name))
        .map(|entry| entry.pid)
        .collect();
    terminate_process_trees(&table, targets)
}

pub fn terminate_by_path(path: &Path) -> usize {
    let targets = pids_by_path(path);
    if targets.is_empty() {
        return 0;
    }
    terminate_process_trees(&snapshot(), targets)
}

fn terminate_process_trees(table: &[SnapshotEntry], mut targets: Vec<u32>) -> usize {
    let mut index = 0;
    while index < targets.len() {
        let parent = targets[index];
        index += 1;
        for entry in table {
            if entry.parent == parent && entry.pid != 0 && !targets.contains(&entry.pid) {
                targets.push(entry.pid);
            }
        }
    }

    let mut killed = 0;
    for pid in targets {
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                if TerminateProcess(handle, 1).is_ok() {
                    killed += 1;
                }
                let _ = CloseHandle(handle);
            }
        }
    }
    killed
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/process/discovery__tests.rs"]
mod tests;
