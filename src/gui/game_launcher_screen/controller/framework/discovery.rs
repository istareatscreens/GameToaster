use super::descriptor::DiscoveredController;
use super::registry::AdapterRegistry;

pub(in crate::gui::game_launcher_screen) fn discover() -> Result<Vec<DiscoveredController>, String>
{
    AdapterRegistry::with_default_adapters().discover()
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/discovery__tests.rs"]
mod tests;
