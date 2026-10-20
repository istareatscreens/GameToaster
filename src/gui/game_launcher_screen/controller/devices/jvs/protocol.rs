use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const SYNC: u8 = 0xE0;
pub(crate) const MARK: u8 = 0xD0;
pub(crate) const BROADCAST: u8 = 0xFF;
pub(crate) const STATUS_OK: u8 = 0x01;
pub(crate) const REPORT_OK: u8 = 0x01;
pub(crate) const SWITCH_IDLE: u16 = 0x0DF8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Frame {
    pub destination: u8,
    pub payload: Vec<u8>,
}

pub(crate) fn encode(destination: u8, payload: &[u8]) -> Result<Vec<u8>, String> {
    let length = payload
        .len()
        .checked_add(1)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| "JVS payload is too large".to_string())?;
    let mut plain = Vec::with_capacity(payload.len() + 4);
    plain.extend([SYNC, destination, length]);
    plain.extend_from_slice(payload);
    let checksum = plain[1..]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    plain.push(checksum);

    let mut wire = Vec::with_capacity(plain.len() * 2);
    wire.push(SYNC);
    for byte in plain.into_iter().skip(1) {
        if byte == SYNC || byte == MARK {
            wire.extend([MARK, byte.wrapping_sub(1)]);
        } else {
            wire.push(byte);
        }
    }
    Ok(wire)
}

pub(crate) fn decode(wire: &[u8]) -> Result<Frame, String> {
    if wire.first() != Some(&SYNC) {
        return Err("JVS frame has no sync byte".into());
    }
    let mut plain = vec![SYNC];
    let mut escaped = false;
    for byte in wire.iter().copied().skip(1) {
        if escaped {
            plain.push(byte.wrapping_add(1));
            escaped = false;
        } else if byte == MARK {
            escaped = true;
        } else {
            plain.push(byte);
        }
    }
    if escaped {
        return Err("JVS frame ends with an escape marker".into());
    }
    if plain.len() < 4 {
        return Err("JVS frame is shorter than sync/destination/length/checksum".into());
    }
    let expected = 3usize + plain[2] as usize;
    if plain.len() != expected {
        return Err(format!(
            "JVS length field expects {expected} bytes but received {}",
            plain.len()
        ));
    }
    let checksum_index = plain.len() - 1;
    let checksum = plain[1..checksum_index]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    if checksum != plain[checksum_index] {
        return Err("JVS checksum mismatch".into());
    }
    Ok(Frame {
        destination: plain[1],
        payload: plain[3..checksum_index].to_vec(),
    })
}

pub(crate) fn reset_request() -> Result<Vec<u8>, String> {
    encode(BROADCAST, &[0xF0, 0xD9])
}

pub(crate) fn assign_address_request(address: u8) -> Result<Vec<u8>, String> {
    encode(BROADCAST, &[0xF1, address])
}

pub(crate) fn command_request(address: u8, command: u8) -> Result<Vec<u8>, String> {
    encode(address, &[command])
}

pub(crate) fn read_switches_request(address: u8, players: u8) -> Result<Vec<u8>, String> {
    encode(address, &[0x20, players.max(1), 0x02])
}

pub(crate) const CMD_READ_COIN: u8 = 0x21;

pub(crate) const CMD_COIN_DECREASE: u8 = 0x30;

pub(crate) const CMD_WRITE_GPIO: u8 = 0x32;

pub(crate) fn read_coin_request(address: u8, slots: u8) -> Result<Vec<u8>, String> {
    encode(address, &[CMD_READ_COIN, slots.max(1)])
}

pub(crate) fn decode_coin_response(frame: &Frame, slots: usize) -> Result<Vec<(u8, u8)>, String> {
    if frame.destination != 0 {
        return Err("JVS response is not addressed to the master".into());
    }
    if frame.payload.len() < 2 + slots * 2
        || frame.payload[0] != STATUS_OK
        || frame.payload[1] != REPORT_OK
    {
        return Err("JVS coin response has an invalid status or payload".into());
    }
    Ok((0..slots)
        .map(|slot| {
            let hi = 2 + slot * 2;
            (frame.payload[hi], frame.payload[hi + 1])
        })
        .collect())
}

#[allow(dead_code)]
pub(crate) fn coin_decrease_request(address: u8, slot: u8, amount: u16) -> Result<Vec<u8>, String> {
    encode(
        address,
        &[
            CMD_COIN_DECREASE,
            slot,
            (amount >> 8) as u8,
            (amount & 0xFF) as u8,
        ],
    )
}

#[allow(dead_code)]
pub(crate) fn write_gpio_request(address: u8, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let count = u8::try_from(bytes.len()).map_err(|_| "too many GPIO bytes".to_string())?;
    let mut payload = Vec::with_capacity(bytes.len() + 2);
    payload.push(CMD_WRITE_GPIO);
    payload.push(count);
    payload.extend_from_slice(bytes);
    encode(address, &payload)
}

pub(crate) fn decode_switch_response(
    frame: &Frame,
    player_index: usize,
) -> Result<ButtonWord, String> {
    if frame.destination != 0 {
        return Err("JVS response is not addressed to the master".into());
    }
    let high = 3 + player_index * 2;
    if frame.payload.len() < high + 2
        || frame.payload[0] != STATUS_OK
        || frame.payload[1] != REPORT_OK
    {
        return Err("JVS switch response has an invalid status or payload".into());
    }
    let switches = u16::from_be_bytes([frame.payload[high], frame.payload[high + 1]]);
    let mut word = ButtonWord::EMPTY;
    for (bit, button) in [
        (11, Button::Ba1),
        (10, Button::Ba2),
        (8, Button::Ba3),
        (7, Button::Ba4),
        (6, Button::Ba5),
        (5, Button::Ba6),
        (4, Button::Ba7),
        (3, Button::Ba8),
    ] {
        if SWITCH_IDLE & (1 << bit) != 0 && switches & (1 << bit) == 0 {
            word.set(button);
        }
    }
    if switches & (1 << 14) != 0 {
        word.set(Button::Select);
    }
    Ok(word)
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/jvs/protocol__tests.rs"]
mod tests;
