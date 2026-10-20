use super::super::controller::{Button, ButtonWord, ManagerEvent};
use super::super::domain::command;
use super::super::ui::settings::{LiveButtons, TestStatus};
use super::super::{ControllerLauncherAction, ControllerScreenConfig};
use super::status::ControllerStatus;
use super::ControllerLauncher;

impl ControllerLauncher {
    pub(crate) fn poll(
        &mut self,
        config: &ControllerScreenConfig,
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
                    if let Some(command) = self.manager.command_for(button) {
                        command::apply(command, config, &mut self.games, &mut actions);
                    }
                }
                ManagerEvent::Edge { pressed: false, .. } => {}
            }
        }
        actions
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
