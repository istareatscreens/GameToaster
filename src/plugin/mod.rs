pub mod catalog;
pub mod deployment;
pub mod lifecycle;
pub mod manifest;
pub mod paths;
pub mod runtime;
pub mod workflow;

#[cfg(test)]
#[path = "../../tests/unit/plugin/mod__fixtures.rs"]
pub(crate) mod fixtures;

#[cfg(test)]
#[path = "../../tests/unit/plugin/mod__tests.rs"]
mod tests;
