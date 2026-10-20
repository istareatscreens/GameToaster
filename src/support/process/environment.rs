use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;

pub fn environment_block(overrides: &[(OsString, OsString)]) -> Vec<u16> {
    let mut environment: BTreeMap<String, (OsString, OsString)> = std::env::vars_os()
        .map(|(key, value)| (key.to_string_lossy().to_ascii_uppercase(), (key, value)))
        .collect();
    for (key, value) in overrides {
        environment.insert(
            key.to_string_lossy().to_ascii_uppercase(),
            (key.clone(), value.clone()),
        );
    }
    let mut block = Vec::new();
    for (_, (key, value)) in environment {
        let mut entry = key;
        entry.push("=");
        entry.push(value);
        block.extend(entry.encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/environment__tests.rs"]
mod tests;
