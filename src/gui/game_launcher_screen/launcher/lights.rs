use super::super::controller::{LightsOutcome, LIGHTS_TIMING};
use super::super::ControllerScreenConfig;
use super::ControllerLauncher;

impl ControllerLauncher {
    pub(crate) fn turn_lights_off_after_game(&mut self, config: &ControllerScreenConfig) {
        if !config.enabled {
            return;
        }
        let Some(request) = self.manager.lights_request(&config.controller) else {
            return;
        };
        let log = self.diagnostics.log();
        let lights = request.lights.name();
        let ports = request.ports.to_owned();
        let reported_ports = ports.clone();
        let started = self
            .lights_off
            .start(request.lights, ports, LIGHTS_TIMING, move |outcome| {
                log.event(
                    "controller-lights-off",
                    &outcome_fields(lights, &reported_ports, &outcome),
                );
            });
        if let Err(error) = started {
            self.diagnostics.error("lights-off", &error);
        }
    }

    pub(crate) fn cancel_lights_off(&self) {
        self.lights_off.cancel();
    }
}

fn outcome_fields(
    lights: &str,
    ports: &str,
    outcome: &LightsOutcome,
) -> Vec<(&'static str, String)> {
    let mut fields = vec![("lights", lights.to_owned()), ("ports", ports.to_owned())];
    match outcome {
        LightsOutcome::TurnedOff { attempts } => {
            fields.push(("outcome", "turned-off".into()));
            fields.push(("attempts", attempts.to_string()));
        }
        LightsOutcome::Cancelled => fields.push(("outcome", "cancelled-by-launch".into())),
        LightsOutcome::GaveUp { attempts } => {
            fields.push(("outcome", "port-still-in-use".into()));
            fields.push(("attempts", attempts.to_string()));
        }
        LightsOutcome::Failed(error) => {
            fields.push(("outcome", "failed".into()));
            fields.push(("error", error.clone()));
        }
    }
    fields
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/lights__tests.rs"]
mod tests;
