use std::collections::BTreeMap;
use std::sync::Arc;

use crate::game_launcher_screen::ControllerConfig;

use super::adapter::ControllerAdapter;
use super::descriptor::{ControllerDriverDescriptor, DiscoveredController};
use super::lights::{self, LightsRequest};

#[derive(Default)]
pub(in crate::gui::game_launcher_screen) struct AdapterRegistry {
    adapters: BTreeMap<&'static str, Arc<dyn ControllerAdapter>>,
}

impl AdapterRegistry {
    pub fn with_default_adapters() -> Self {
        let mut registry = Self::default();
        for adapter in super::super::devices::builtin_adapters() {
            registry
                .register(adapter)
                .expect("built-in controller driver descriptors must be valid and unique");
        }
        registry
    }

    pub(in crate::gui::game_launcher_screen::controller) fn register(
        &mut self,
        adapter: Arc<dyn ControllerAdapter>,
    ) -> Result<(), String> {
        let descriptor = adapter.descriptor();
        descriptor.default_config().validate_shape()?;
        let mut option_keys = std::collections::BTreeSet::new();
        for option in descriptor.options {
            if !option_keys.insert(option.key) {
                return Err(format!(
                    "controller driver '{}' defines option '{}' more than once",
                    descriptor.id, option.key
                ));
            }
        }
        for (index, binding) in descriptor.bindings.iter().enumerate() {
            if descriptor.bindings[..index]
                .iter()
                .any(|earlier| earlier.button == binding.button)
            {
                return Err(format!(
                    "controller driver '{}' binds button {} more than once",
                    descriptor.id,
                    binding.button.label()
                ));
            }
        }
        super::lights::check_declaration(descriptor)?;
        descriptor.validate(&descriptor.default_config())?;
        if self.adapters.contains_key(descriptor.id) {
            return Err(format!(
                "controller driver '{}' is already registered",
                descriptor.id
            ));
        }
        self.adapters.insert(descriptor.id, adapter);
        Ok(())
    }

    pub fn descriptors(&self) -> Vec<&'static ControllerDriverDescriptor> {
        self.adapters
            .values()
            .map(|adapter| adapter.descriptor())
            .collect()
    }

    pub fn default_config(&self, driver_id: &str) -> Result<ControllerConfig, String> {
        self.get(driver_id)
            .map(|adapter| adapter.descriptor().default_config())
            .ok_or_else(|| format!("controller driver '{driver_id}' is not registered"))
    }

    pub fn validate(&self, config: &ControllerConfig) -> Result<(), String> {
        self.get(&config.driver_id)
            .ok_or_else(|| format!("controller driver '{}' is not registered", config.driver_id))?
            .validate(config)
    }

    pub fn discover(&self) -> Result<Vec<DiscoveredController>, String> {
        let mut discovered = Vec::new();
        for adapter in self.adapters.values() {
            discovered.extend(adapter.discover()?);
        }
        Ok(discovered)
    }

    pub fn lights_request<'a>(&self, config: &'a ControllerConfig) -> Option<LightsRequest<'a>> {
        lights::request(self.get(&config.driver_id)?.descriptor(), config)
    }

    pub(in crate::gui::game_launcher_screen::controller) fn get(
        &self,
        driver_id: &str,
    ) -> Option<Arc<dyn ControllerAdapter>> {
        self.adapters.get(driver_id).cloned()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/registry__tests.rs"]
mod tests;
