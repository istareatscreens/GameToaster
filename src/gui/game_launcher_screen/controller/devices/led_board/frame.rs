const SYNC: u8 = 0xE0;
const ESCAPE: u8 = 0xD0;

const BOARD: u8 = 0x11;

const HOST: u8 = 0x01;

pub(super) const RESET: u8 = 0x10;

pub(super) const SET_LEDS: u8 = 0x32;

pub(super) const SET_CABINET_LIGHTS: u8 = 0x39;

pub(super) const APPLY: u8 = 0x3C;

pub(super) fn request(command: u8, payload: &[u8]) -> Vec<u8> {
    let length = u8::try_from(payload.len() + 1).expect("an LED board payload fits in a frame");
    let mut body = vec![BOARD, HOST, length, command];
    body.extend_from_slice(payload);
    let sum = body.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    body.push(sum);
    let mut frame = vec![SYNC];
    for byte in body {
        if byte == SYNC || byte == ESCAPE {
            frame.extend([ESCAPE, byte - 1]);
        } else {
            frame.push(byte);
        }
    }
    frame
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/led_board/frame__tests.rs"]
mod tests;
