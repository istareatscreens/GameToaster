use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::support::win::wide;

pub fn command_line(executable: &Path, arguments: &[OsString]) -> Vec<u16> {
    let mut command = quote_argument(executable.as_os_str());
    for argument in arguments {
        command.push(' ');
        command.push_str(&quote_argument(argument));
    }
    wide(&command)
}

pub fn quote_argument(argument: &OsStr) -> String {
    let text = argument.to_string_lossy();
    if !text.is_empty() && !text.chars().any(|c| c.is_whitespace() || c == '"') {
        return text.into_owned();
    }
    let mut output = String::from("\"");
    let mut slashes = 0;
    for character in text.chars() {
        match character {
            '\\' => slashes += 1,
            '"' => {
                output.push_str(&"\\".repeat(slashes * 2 + 1));
                output.push('"');
                slashes = 0;
            }
            _ => {
                output.push_str(&"\\".repeat(slashes));
                slashes = 0;
                output.push(character);
            }
        }
    }
    output.push_str(&"\\".repeat(slashes * 2));
    output.push('"');
    output
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/command_line__tests.rs"]
mod tests;
