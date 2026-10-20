use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/support/win__tests.rs"]
mod tests;
