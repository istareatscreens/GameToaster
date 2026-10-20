pub(super) mod adapter;
pub(super) mod audit;
pub(super) mod buttons;
pub(super) mod descriptor;
pub(super) mod discovery;
pub(super) mod edges;
pub(super) mod events;
pub(super) mod lights;
pub(super) mod manager;
pub(super) mod registry;

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/mod__fakes.rs"]
mod fakes;
