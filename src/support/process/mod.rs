pub mod command_line;
pub mod console_policy;
pub mod environment;
pub mod handle;
pub mod image;
pub mod interpreter;
pub mod invocation;
pub mod pause;
pub mod snapshot;
pub mod tree;

pub const ENDED_BY_LAUNCHER: u32 = 1;

#[cfg(test)]
#[path = "../../../tests/unit/support/process/mod__fixtures.rs"]
pub(crate) mod fixtures;
