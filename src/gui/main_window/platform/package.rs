use windows::core::PWSTR;
use windows::Win32::Foundation::{
    APPMODEL_ERROR_NO_PACKAGE, ERROR_INSUFFICIENT_BUFFER, WIN32_ERROR,
};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFamilyName;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallKind {
    Portable,
    Packaged,
}

impl InstallKind {
    pub fn current() -> Result<Self, String> {
        let mut length = 0u32;
        let status = unsafe { GetCurrentPackageFamilyName(&mut length, PWSTR::null()) };
        classify(status)
    }
}

fn classify(status: WIN32_ERROR) -> Result<InstallKind, String> {
    if status == APPMODEL_ERROR_NO_PACKAGE {
        Ok(InstallKind::Portable)
    } else if status == ERROR_INSUFFICIENT_BUFFER {
        Ok(InstallKind::Packaged)
    } else {
        Err(format!(
            "cannot tell whether GameToaster runs from a package: GetCurrentPackageFamilyName returned {}",
            status.0
        ))
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/platform/package__tests.rs"]
mod tests;
