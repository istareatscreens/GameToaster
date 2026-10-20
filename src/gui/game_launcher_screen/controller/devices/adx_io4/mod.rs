use std::sync::Arc;

use super::super::framework::descriptor::{
    ControllerDriverDescriptor, ControllerOptionDescriptor, ControllerOptionKind,
};
use super::super::framework::lights::TURN_OFF_ON_GAME_CLOSE;
use super::super::transport::hid::HidAdapter;
use super::led_board;
use super::ring_bindings::RING_BINDINGS;
use report::{AdxIo4Decoder, DEFAULT_PID, DEFAULT_VID, REPORT_SIZE};

mod report;

const IDENTITIES: &[(u16, u16)] = &[(DEFAULT_VID, DEFAULT_PID)];
const OPTIONS: &[ControllerOptionDescriptor] = &[
    ControllerOptionDescriptor {
        key: "vid",
        label_key: "controller.type.vid",
        default: "0CA3",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
    ControllerOptionDescriptor {
        key: "pid",
        label_key: "controller.type.pid",
        default: "0021",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
    led_board::ports_option("COM21"),
    TURN_OFF_ON_GAME_CLOSE,
];
pub(super) static DESCRIPTOR: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "adx-io4",
    label_key: "controller.type.adx_io4",
    recommended_layout: "washing-machine",
    options: OPTIONS,
    lights: Some(led_board::DESCRIPTOR),
    bindings: RING_BINDINGS,
};

pub(in crate::gui::game_launcher_screen::controller) fn adapter() -> HidAdapter {
    HidAdapter::new(&DESCRIPTOR, IDENTITIES, REPORT_SIZE, Arc::new(decoder()))
}

#[cfg(test)]
pub(in crate::gui::game_launcher_screen::controller) fn identity() -> (u16, u16, usize) {
    (DEFAULT_VID, DEFAULT_PID, REPORT_SIZE)
}

pub(in crate::gui::game_launcher_screen::controller) fn decoder() -> AdxIo4Decoder {
    AdxIo4Decoder
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/adx_io4/mod__tests.rs"]
mod tests;
