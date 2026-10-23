mod package_task;
mod run_key;

use std::ffi::OsString;

use super::package::InstallKind;

pub fn is_enabled() -> bool {
    match InstallKind::current() {
        Ok(InstallKind::Portable) => run_key::is_enabled(),
        Ok(InstallKind::Packaged) => package_task::is_enabled(),
        Err(_) => false,
    }
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    match InstallKind::current()? {
        InstallKind::Portable => run_key::set_enabled(enabled),
        InstallKind::Packaged => package_task::set_enabled(enabled),
    }
}

pub fn repair_entry() -> Result<(), String> {
    match InstallKind::current()? {
        InstallKind::Portable => run_key::repair(),
        InstallKind::Packaged => Ok(()),
    }
}

pub fn launched_at_sign_in(args: &[OsString]) -> bool {
    match InstallKind::current() {
        Ok(InstallKind::Portable) => run_key::launched_at_sign_in(args),
        Ok(InstallKind::Packaged) => package_task::launched_at_sign_in(),
        Err(_) => false,
    }
}
