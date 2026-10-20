use std::collections::BTreeSet;

use rust_i18n::t;

use crate::game_launcher_screen::ControllerConfig;
use crate::gui::game_launcher_screen::domain::command::LauncherCommand;

use super::buttons::Button;
use super::lights::{self, LightsDescriptor};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct ControllerChoice {
    pub value: &'static str,
    pub label_key: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum ControllerOptionKind {
    HexU16 { non_zero: bool },
    Text { allow_empty: bool },
    Boolean,
    Choice(&'static [ControllerChoice]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct ControllerOptionDescriptor {
    pub key: &'static str,
    pub label_key: &'static str,
    pub default: &'static str,
    pub kind: ControllerOptionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct ButtonBinding {
    pub button: Button,
    pub command: LauncherCommand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct ControllerDriverDescriptor {
    pub id: &'static str,
    pub label_key: &'static str,
    pub recommended_layout: &'static str,
    pub options: &'static [ControllerOptionDescriptor],

    pub lights: Option<LightsDescriptor>,

    pub bindings: &'static [ButtonBinding],
}

impl ControllerDriverDescriptor {
    pub fn default_config(self) -> ControllerConfig {
        ControllerConfig::new(
            self.id,
            self.options
                .iter()
                .map(|option| (option.key, option.default)),
        )
    }

    pub fn validate(self, config: &ControllerConfig) -> Result<(), String> {
        config.validate_shape()?;
        if config.driver_id != self.id {
            return Err(format!(
                "controller config selects '{}' but '{}' was asked to validate it",
                config.driver_id, self.id
            ));
        }
        let known: BTreeSet<&str> = self.options.iter().map(|option| option.key).collect();
        for key in config.options.keys() {
            if !known.contains(key.as_str()) {
                return Err(format!(
                    "controller driver '{}' does not define option '{key}'",
                    self.id
                ));
            }
        }
        for option in self.options {
            let value = config.options.get(option.key).ok_or_else(|| {
                format!(
                    "controller driver '{}' requires option '{}'",
                    self.id, option.key
                )
            })?;
            validate_option(option, value)?;
        }
        lights::validate(&self, config)
    }

    pub fn command_for(self, button: Button) -> Option<LauncherCommand> {
        self.bindings
            .iter()
            .find(|binding| binding.button == button)
            .map(|binding| binding.command)
    }
}

fn validate_option(option: &ControllerOptionDescriptor, value: &str) -> Result<(), String> {
    match option.kind {
        ControllerOptionKind::HexU16 { non_zero } => {
            let parsed = u16::from_str_radix(value.trim(), 16)
                .map_err(|_| t!("controller.validation.hex", field => option.key).to_string())?;
            if non_zero && parsed == 0 {
                return Err(
                    t!("controller.validation.nonzero_hid", field => option.key).to_string()
                );
            }
        }
        ControllerOptionKind::Text { allow_empty } => {
            if !allow_empty && value.trim().is_empty() {
                return Err(t!("controller.validation.not_empty", field => option.key).to_string());
            }
        }
        ControllerOptionKind::Boolean => {
            if value != "true" && value != "false" {
                return Err(t!("controller.validation.boolean", field => option.key).to_string());
            }
        }
        ControllerOptionKind::Choice(choices) => {
            if !choices.iter().any(|choice| choice.value == value) {
                return Err(t!(
                    "controller.validation.unsupported_value",
                    field => option.key,
                    value => value
                )
                .to_string());
            }
        }
    }
    Ok(())
}

pub(in crate::gui::game_launcher_screen::controller) fn required_option<'a>(
    config: &'a ControllerConfig,
    key: &str,
) -> Result<&'a str, String> {
    config
        .options
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("controller option '{key}' is missing"))
}

pub(in crate::gui::game_launcher_screen::controller) fn hex_u16_option(
    config: &ControllerConfig,
    key: &str,
) -> Result<u16, String> {
    u16::from_str_radix(required_option(config, key)?.trim(), 16)
        .map_err(|_| format!("controller option '{key}' is not a hexadecimal 16-bit value"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct DiscoveredController {
    pub config: ControllerConfig,
    pub label_key: &'static str,
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/descriptor__tests.rs"]
mod tests;
