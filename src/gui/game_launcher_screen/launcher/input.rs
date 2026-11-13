use super::super::controller::{Button, ButtonWord, ManagerEvent};
use super::super::domain::command;
use super::super::domain::command::LauncherCommand;
use super::super::ui::settings::{LiveButtons, TestStatus};
use super::super::{ControllerLauncherAction, ControllerScreenConfig};
use super::status::ControllerStatus;
use super::ControllerLauncher;

impl ControllerLauncher {
    pub(crate) fn poll(
        &mut self,
        config: &ControllerScreenConfig,
        launch_cancellable: bool,
    ) -> Vec<ControllerLauncherAction> {
        self.poll_monitor_watcher(config);

        self.tick_connect();
        let mut actions = Vec::new();
        for event in self.manager.poll() {
            match event {
                ManagerEvent::Connected => {
                    self.status = ControllerStatus::Connected;
                    if let Some(settings) = &mut self.settings {
                        if self.test_pending {
                            settings.test_status = TestStatus::ConnectedWaiting;
                        }
                    }
                }
                ManagerEvent::Snapshot(snapshot) => {
                    if self.test_pending {
                        if let Some(settings) = &mut self.settings {
                            settings.test_status = TestStatus::Passed;
                            settings.live_buttons = LiveButtons::Report(button_labels(snapshot));
                        }
                    }
                }
                ManagerEvent::Error(error) => {
                    self.status = ControllerStatus::Error(error.clone());
                    self.diagnostics.error("controller-io", &error);
                    if let Some(settings) = &mut self.settings {
                        if self.test_pending {
                            settings.test_status = TestStatus::Failed(error.clone());
                        }
                    }
                }
                ManagerEvent::Disconnected => self.status = ControllerStatus::Disconnected,
                ManagerEvent::Edge {
                    button,
                    pressed: true,
                } => {
                    let command = self.manager.command_for(button);
                    match press_duty(self.test_pending, self.launch_in_progress()) {
                        PressDuty::Readout => {}
                        PressDuty::CancelLaunch => {
                            if command == Some(LauncherCommand::Launch)
                                && launch_cancellable
                                && self.cancel_arming.may_cancel()
                            {
                                actions.push(ControllerLauncherAction::CancelLaunch);
                            }
                        }
                        PressDuty::Command => {
                            if let Some(command) = command {
                                command::apply(command, config, &mut self.games, &mut actions);
                            }
                        }
                    }
                }
                ManagerEvent::Edge {
                    button,
                    pressed: false,
                } => {
                    if self.manager.command_for(button) == Some(LauncherCommand::Launch) {
                        if let Some(started) = self.launch_started {
                            self.cancel_arming.released(started.elapsed());
                        }
                    }
                }
            }
        }
        actions
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PressDuty {
    Readout,

    CancelLaunch,

    Command,
}

fn press_duty(testing: bool, launching: bool) -> PressDuty {
    if testing {
        PressDuty::Readout
    } else if launching {
        PressDuty::CancelLaunch
    } else {
        PressDuty::Command
    }
}

fn button_labels(snapshot: ButtonWord) -> String {
    let labels: Vec<&'static str> = Button::ALL
        .into_iter()
        .filter(|button| snapshot.contains(*button))
        .map(Button::label)
        .collect();
    if labels.is_empty() {
        "none".into()
    } else {
        labels.join(", ")
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/input__tests.rs"]
mod tests;
