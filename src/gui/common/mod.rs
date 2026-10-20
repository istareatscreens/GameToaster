pub mod coin;
pub mod components;
pub mod game_state;
pub mod hash;
pub(crate) mod message;
pub mod theme;

#[cfg(test)]
#[path = "../../../tests/unit/gui/common/mod__harness.rs"]
pub(crate) mod harness;

#[cfg(test)]
#[path = "../../../tests/unit/gui/common/mod__tests.rs"]
mod tests;
