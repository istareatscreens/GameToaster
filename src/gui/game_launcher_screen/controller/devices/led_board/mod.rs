mod frame;

use std::thread;
use std::time::Duration;

use rust_i18n::t;

use super::super::framework::audit::ResourceAudit;
use super::super::framework::descriptor::{ControllerOptionDescriptor, ControllerOptionKind};
use super::super::framework::lights::{ControllerLights, LightsDescriptor, LightsError};
use super::super::transport::serial::{com_port_path, SerialOpenError, SerialTransport};
use frame::{request, APPLY, RESET, SET_CABINET_LIGHTS, SET_LEDS};

pub(super) static LIGHTS: LedBoardLights = LedBoardLights;

const PORTS_KEY: &str = "led_ports";

pub(super) const DESCRIPTOR: LightsDescriptor = LightsDescriptor {
    ports_option: PORTS_KEY,
    lights: &LIGHTS,
};

pub(super) const fn ports_option(default: &'static str) -> ControllerOptionDescriptor {
    ControllerOptionDescriptor {
        key: PORTS_KEY,
        label_key: "controller.lights.ports",
        default,
        kind: ControllerOptionKind::Text { allow_empty: true },
    }
}

const WRITE_TIMEOUT: Duration = Duration::from_millis(1000);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Step {
    pub bytes: Vec<u8>,
    pub pause_after: Duration,
}

pub(super) fn turn_off_sequence() -> Vec<Step> {
    let step = |bytes: Vec<u8>, pause_ms: u64| Step {
        bytes,
        pause_after: Duration::from_millis(pause_ms),
    };
    vec![
        step(request(RESET, &[]), 50),
        step(request(RESET, &[]), 50),
        step(request(RESET, &[]), 150),
        step(
            request(SET_LEDS, &[0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00]),
            10,
        ),
        step(request(SET_CABINET_LIGHTS, &[0x00, 0x00, 0x00]), 10),
        step(request(APPLY, &[]), 50),
    ]
}

pub(super) trait LightPort {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String>;
}

impl LightPort for SerialTransport {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.write_all(bytes, WRITE_TIMEOUT)
    }
}

fn port_paths(ports: &str) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for name in ports
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let path = com_port_path(name)
            .ok_or_else(|| t!("controller.validation.led_port", port => name).to_string())?;
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub(super) fn turn_off_ports(
    paths: &[String],
    mut open: impl FnMut(&str) -> Result<Box<dyn LightPort>, SerialOpenError>,
    mut pause: impl FnMut(Duration),
) -> Result<(), LightsError> {
    let mut busy = false;
    let mut failures = Vec::new();
    for path in paths {
        let mut port = match open(path) {
            Ok(port) => port,
            Err(SerialOpenError::InUse) => {
                busy = true;
                continue;
            }
            Err(SerialOpenError::Failed(message)) => {
                failures.push(message);
                continue;
            }
        };
        for step in turn_off_sequence() {
            if let Err(message) = port.write(&step.bytes) {
                failures.push(format!("{path}: {message}"));
                break;
            }
            pause(step.pause_after);
        }
    }
    if busy {
        Err(LightsError::Busy)
    } else if failures.is_empty() {
        Ok(())
    } else {
        Err(LightsError::Failed(failures.join("; ")))
    }
}

pub(super) struct LedBoardLights;

impl ControllerLights for LedBoardLights {
    fn name(&self) -> &'static str {
        "led-board"
    }

    fn validate_ports(&self, ports: &str) -> Result<(), String> {
        port_paths(ports).map(drop)
    }

    fn turn_off(&self, ports: &str) -> Result<(), LightsError> {
        let paths = port_paths(ports).map_err(LightsError::Failed)?;
        let audit = ResourceAudit::new();
        turn_off_ports(
            &paths,
            |path| {
                SerialTransport::open(path, &audit)
                    .map(|transport| Box::new(transport) as Box<dyn LightPort>)
            },
            thread::sleep,
        )
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/led_board/mod__tests.rs"]
mod tests;
