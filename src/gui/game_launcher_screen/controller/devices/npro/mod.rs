use std::sync::Arc;

use windows::core::GUID;

use super::super::framework::descriptor::{
    ControllerDriverDescriptor, ControllerOptionDescriptor, ControllerOptionKind,
};
use super::super::transport::winusb::WinUsbAdapter;
use super::ring_bindings::RING_BINDINGS;
use report::{NproDecoder, NPRO_PID, NPRO_PID_P2, NPRO_VID, REPORT_SIZE};

mod report;

const INTERFACE_CLASS: GUID = GUID::from_u128(0x7F2A9C41_5E3B_4D8A_9F16_3C7E5B2D9A48);
const INTERFACE_NUMBER: u8 = 0;

const IDENTITIES: &[(u16, u16)] = &[(NPRO_VID, NPRO_PID), (NPRO_VID, NPRO_PID_P2)];
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
        default: "5751",
        kind: ControllerOptionKind::HexU16 { non_zero: true },
    },
];
pub(super) static DESCRIPTOR: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "npro",
    label_key: "controller.type.npro",
    recommended_layout: "washing-machine",
    options: OPTIONS,
    lights: None,
    bindings: RING_BINDINGS,
};

pub(in crate::gui::game_launcher_screen::controller) fn adapter() -> WinUsbAdapter {
    WinUsbAdapter::new(
        &DESCRIPTOR,
        IDENTITIES,
        INTERFACE_NUMBER,
        Some(INTERFACE_CLASS),
        REPORT_SIZE,
        Arc::new(decoder()),
    )
}

pub(in crate::gui::game_launcher_screen::controller) fn decoder() -> NproDecoder {
    NproDecoder
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/npro/mod__tests.rs"]
mod tests;
