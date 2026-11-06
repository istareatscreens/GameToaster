use std::sync::Arc;

use super::super::framework::descriptor::{
    ControllerDriverDescriptor, ControllerOptionDescriptor, ControllerOptionKind,
};
use super::super::transport::hid::HidAdapter;
use super::ring_bindings::RING_BINDINGS;
use report::{DaoDecoder, DEFAULT_PID, DEFAULT_VID, REPORT_SIZE};

mod report;

const IDENTITIES: &[(u16, u16)] = &[(DEFAULT_VID, DEFAULT_PID)];
const OPTIONS: &[ControllerOptionDescriptor] = &[
    ControllerOptionDescriptor {
        key: "vid",
        label_key: "controller.type.vid",
        default: "0E8F",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
    ControllerOptionDescriptor {
        key: "pid",
        label_key: "controller.type.pid",
        default: "1224",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
];
pub(super) static DESCRIPTOR: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "dao",
    label_key: "controller.type.dao",
    recommended_layout: "washing-machine",
    options: OPTIONS,
    lights: None,
    bindings: RING_BINDINGS,
};

pub(in crate::gui::game_launcher_screen::controller) fn adapter() -> HidAdapter {
    HidAdapter::new(&DESCRIPTOR, IDENTITIES, REPORT_SIZE, Arc::new(decoder()))
}

#[cfg(test)]
pub(in crate::gui::game_launcher_screen::controller) fn identity() -> (u16, u16, usize) {
    (DEFAULT_VID, DEFAULT_PID, REPORT_SIZE)
}

pub(in crate::gui::game_launcher_screen::controller) fn decoder() -> DaoDecoder {
    DaoDecoder
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/dao/mod__tests.rs"]
mod tests;
