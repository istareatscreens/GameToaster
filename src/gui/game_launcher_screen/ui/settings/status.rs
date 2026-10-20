use rust_i18n::t;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum TestStatus {
    NotTested,
    Connecting,
    ConnectedWaiting,
    Passed,
    Failed(String),
    ReleaseFailed(String),
    CouldNotStop(String),
    RestoreFailed(String),
    Stopped,
    Detected(String),
    NoneDetected,
    MultipleDetected(usize),
}

impl TestStatus {
    pub fn resolve(&self) -> String {
        match self {
            Self::NotTested => t!("controller.test.not_tested").to_string(),
            Self::Connecting => t!("controller.test.connecting").to_string(),
            Self::ConnectedWaiting => t!("controller.test.connected_waiting").to_string(),
            Self::Passed => t!("controller.test.passed").to_string(),
            Self::Failed(d) => t!("controller.test.failed", detail => d).to_string(),
            Self::ReleaseFailed(d) => t!("controller.test.release_failed", detail => d).to_string(),
            Self::CouldNotStop(d) => t!("controller.test.could_not_stop", detail => d).to_string(),
            Self::RestoreFailed(d) => t!("controller.test.restore_failed", detail => d).to_string(),
            Self::Stopped => t!("controller.test.stopped").to_string(),
            Self::Detected(label) => t!("controller.test.detected", label => label).to_string(),
            Self::NoneDetected => t!("controller.test.none_detected").to_string(),
            Self::MultipleDetected(count) => {
                t!("controller.test.multiple_detected", count => count).to_string()
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum LiveButtons {
    NoReports,
    Waiting,
    Report(String),
}

impl LiveButtons {
    pub fn resolve(&self) -> String {
        match self {
            Self::NoReports => t!("controller.test.no_reports").to_string(),
            Self::Waiting => t!("controller.test.waiting").to_string(),
            Self::Report(labels) => labels.clone(),
        }
    }
}
