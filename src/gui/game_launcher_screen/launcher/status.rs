use rust_i18n::t;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ControllerStatus {
    Disconnected,
    Connecting,

    Reconnecting,
    ConnectError(String),
    Released,
    Connected,
    Error(String),
    DisplayChanged(String),
}

impl ControllerStatus {
    pub(super) fn resolve(&self) -> String {
        match self {
            Self::Disconnected => t!("controller.status.disconnected").to_string(),
            Self::Connecting | Self::Reconnecting => t!("controller.status.connecting").to_string(),
            Self::ConnectError(detail) => {
                t!("controller.status.connect_error", detail => detail).to_string()
            }
            Self::Released => t!("controller.status.released").to_string(),
            Self::Connected => t!("controller.status.connected").to_string(),
            Self::Error(detail) => t!("controller.status.error", detail => detail).to_string(),
            Self::DisplayChanged(detail) => {
                t!("controller.status.display_changed", detail => detail).to_string()
            }
        }
    }
}
