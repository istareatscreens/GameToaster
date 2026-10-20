pub(in crate::gui::game_launcher_screen) mod job;

use std::fmt;

use rust_i18n::t;

use crate::game_launcher_screen::ControllerConfig;

use super::descriptor::{
    ControllerDriverDescriptor, ControllerOptionDescriptor, ControllerOptionKind,
};

pub(in crate::gui::game_launcher_screen) const TURN_OFF_ON_GAME_CLOSE: ControllerOptionDescriptor =
    ControllerOptionDescriptor {
        key: "turn_lights_off_on_game_close",
        label_key: "controller.lights.turn_off_on_game_close",
        default: "false",
        kind: ControllerOptionKind::Boolean,
    };

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum LightsError {
    Busy,
    Failed(String),
}

impl fmt::Display for LightsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy => formatter.write_str("a lights port is in use by another program"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

pub(in crate::gui::game_launcher_screen) trait ControllerLights:
    Sync
{
    fn name(&self) -> &'static str;

    fn validate_ports(&self, ports: &str) -> Result<(), String>;

    fn turn_off(&self, ports: &str) -> Result<(), LightsError>;
}

#[derive(Clone, Copy)]
pub(in crate::gui::game_launcher_screen) struct LightsDescriptor {
    pub ports_option: &'static str,
    pub lights: &'static dyn ControllerLights,
}

impl PartialEq for LightsDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.ports_option == other.ports_option && self.lights.name() == other.lights.name()
    }
}

impl Eq for LightsDescriptor {}

impl fmt::Debug for LightsDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LightsDescriptor")
            .field("ports_option", &self.ports_option)
            .field("lights", &self.lights.name())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub(in crate::gui::game_launcher_screen) struct LightsRequest<'a> {
    pub lights: &'static dyn ControllerLights,
    pub ports: &'a str,
}

pub(in crate::gui::game_launcher_screen) fn request<'a>(
    descriptor: &ControllerDriverDescriptor,
    config: &'a ControllerConfig,
) -> Option<LightsRequest<'a>> {
    let lights = descriptor.lights?;
    if !switch_on(config) {
        return None;
    }
    let ports = config.options.get(lights.ports_option)?.as_str();
    (!ports.trim().is_empty()).then_some(LightsRequest {
        lights: lights.lights,
        ports,
    })
}

fn switch_on(config: &ControllerConfig) -> bool {
    config
        .options
        .get(TURN_OFF_ON_GAME_CLOSE.key)
        .is_some_and(|value| value == "true")
}

pub(super) fn validate(
    descriptor: &ControllerDriverDescriptor,
    config: &ControllerConfig,
) -> Result<(), String> {
    let Some(lights) = descriptor.lights else {
        return Ok(());
    };
    let ports = config
        .options
        .get(lights.ports_option)
        .map(String::as_str)
        .unwrap_or_default();
    if ports.trim().is_empty() {
        if switch_on(config) {
            return Err(t!("controller.validation.lights_ports").to_string());
        }
        return Ok(());
    }
    lights.lights.validate_ports(ports)
}

pub(super) fn check_declaration(descriptor: &ControllerDriverDescriptor) -> Result<(), String> {
    let lists = |key: &str| descriptor.options.iter().any(|option| option.key == key);
    match descriptor.lights {
        Some(lights) => {
            if !lists(TURN_OFF_ON_GAME_CLOSE.key) {
                return Err(format!(
                    "controller driver '{}' has lights but no '{}' option",
                    descriptor.id, TURN_OFF_ON_GAME_CLOSE.key
                ));
            }
            if !lists(lights.ports_option) {
                return Err(format!(
                    "controller driver '{}' names lights ports option '{}' but does not define it",
                    descriptor.id, lights.ports_option
                ));
            }
        }
        None => {
            if lists(TURN_OFF_ON_GAME_CLOSE.key) {
                return Err(format!(
                    "controller driver '{}' offers '{}' but has no lights",
                    descriptor.id, TURN_OFF_ON_GAME_CLOSE.key
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/framework/lights/mod__tests.rs"]
mod tests;
