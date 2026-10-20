use super::super::super::framework::buttons::ButtonWord;
use super::link::JvsLink;
use super::protocol::{
    assign_address_request, command_request, decode_coin_response, decode_switch_response,
    read_coin_request, read_switches_request, reset_request, Frame, REPORT_OK, STATUS_OK,
};

const NODE_ADDRESS: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum JvsPlayer {
    P1,
    P2,
}

impl JvsPlayer {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "p1" => Ok(Self::P1),
            "p2" => Ok(Self::P2),
            _ => Err(format!("unsupported JVS player '{value}'")),
        }
    }

    fn request_count(self) -> u8 {
        match self {
            Self::P1 => 1,
            Self::P2 => 2,
        }
    }

    fn switch_index(self) -> usize {
        match self {
            Self::P1 => 0,
            Self::P2 => 1,
        }
    }
}

pub(super) struct JvsHost {
    pub(super) player: JvsPlayer,
}

impl JvsHost {
    pub(super) fn initialize(&self, link: &mut dyn JvsLink) -> Result<(), String> {
        link.send_without_reply(&reset_request()?)?;
        link.sense()?;
        require_ok(
            link.transact(&assign_address_request(NODE_ADDRESS)?)?,
            "assign address",
        )?;
        link.sense()?;
        let identity_frame = link.transact(&command_request(NODE_ADDRESS, 0x10)?)?;
        let identity_payload = require_ok(identity_frame, "read identity")?;
        let identity = identity_payload
            .iter()
            .copied()
            .take_while(|byte| *byte != 0)
            .map(char::from)
            .collect::<String>();
        if identity.trim().is_empty() {
            return Err("JVS board returned an empty identity".into());
        }
        single_value(
            link.transact(&command_request(NODE_ADDRESS, 0x11)?)?,
            "command version",
        )?;
        single_value(
            link.transact(&command_request(NODE_ADDRESS, 0x12)?)?,
            "JVS version",
        )?;
        single_value(
            link.transact(&command_request(NODE_ADDRESS, 0x13)?)?,
            "communication version",
        )?;
        let features = require_ok(
            link.transact(&command_request(NODE_ADDRESS, 0x14)?)?,
            "features",
        )?;
        switch_capability(&features)
    }

    pub(super) fn poll(&self, link: &mut dyn JvsLink) -> Result<ButtonWord, String> {
        let response = link.transact(&read_switches_request(
            NODE_ADDRESS,
            self.player.request_count(),
        )?)?;
        decode_switch_response(&response, self.player.switch_index())
    }

    #[allow(dead_code)]
    fn read_coins(&self, link: &mut dyn JvsLink, slots: u8) -> Result<Vec<(u8, u8)>, String> {
        let response = link.transact(&read_coin_request(NODE_ADDRESS, slots)?)?;
        decode_coin_response(&response, slots as usize)
    }
}

fn require_ok(frame: Frame, operation: &str) -> Result<Vec<u8>, String> {
    if frame.destination != 0 || frame.payload.len() < 2 {
        return Err(format!("JVS {operation} response is malformed"));
    }
    if frame.payload[0] != STATUS_OK || frame.payload[1] != REPORT_OK {
        return Err(format!(
            "JVS {operation} failed with status 0x{:02X}/0x{:02X}",
            frame.payload[0], frame.payload[1]
        ));
    }
    Ok(frame.payload[2..].to_vec())
}

fn single_value(frame: Frame, operation: &str) -> Result<u8, String> {
    require_ok(frame, operation)?
        .first()
        .copied()
        .ok_or_else(|| format!("JVS {operation} response omitted its value"))
}

fn switch_capability(features: &[u8]) -> Result<(), String> {
    let mut cursor = 0;
    while cursor < features.len() {
        let feature = features[cursor];
        if feature == 0 {
            break;
        }
        if cursor + 4 > features.len() {
            return Err("JVS feature list is truncated".into());
        }
        if feature == 0x01 {
            let players = features[cursor + 1];
            let switches = features[cursor + 2];
            if players >= 1 && switches >= 8 {
                return Ok(());
            }
            return Err(format!(
                "JVS board exposes only {players} players and {switches} switches per player"
            ));
        }
        cursor += 4;
    }
    Err("JVS board did not advertise player switch input".into())
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/jvs/host__tests.rs"]
mod tests;
