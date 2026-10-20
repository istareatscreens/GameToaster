use std::sync::Arc;

use super::super::framework::descriptor::{
    ControllerDriverDescriptor, ControllerOptionDescriptor, ControllerOptionKind,
};
use super::super::framework::lights::TURN_OFF_ON_GAME_CLOSE;
use super::super::transport::hid::HidAdapter;
use super::led_board;
use super::ring_bindings::RING_BINDINGS;
use report::{AdxHidDecoder, ADX_HID_ALT_VID, ADX_HID_VID, DEFAULT_PID, REPORT_SIZE};

mod report;

const IDENTITIES: &[(u16, u16)] = &[(ADX_HID_VID, DEFAULT_PID), (ADX_HID_ALT_VID, DEFAULT_PID)];
const OPTIONS: &[ControllerOptionDescriptor] = &[
    ControllerOptionDescriptor {
        key: "vid",
        label_key: "controller.type.vid",
        default: "2E3C",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
    ControllerOptionDescriptor {
        key: "pid",
        label_key: "controller.type.pid",
        default: "5750",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
    led_board::ports_option("COM21"),
    TURN_OFF_ON_GAME_CLOSE,
];
pub(super) static DESCRIPTOR: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "adx-hid",
    label_key: "controller.type.adx_hid",
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
    (ADX_HID_VID, DEFAULT_PID, REPORT_SIZE)
}

pub(in crate::gui::game_launcher_screen::controller) fn decoder() -> AdxHidDecoder {
    AdxHidDecoder
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/adx_hid/mod__tests.rs"]
mod tests;
