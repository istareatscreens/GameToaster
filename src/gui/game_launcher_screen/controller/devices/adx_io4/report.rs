use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const DEFAULT_VID: u16 = 0x0CA3;
pub(crate) const DEFAULT_PID: u16 = 0x0021;
pub(crate) const REPORT_SIZE: usize = 64;
pub(crate) const DATA_OFFSET: usize = 26;

#[derive(Default)]
pub(crate) struct AdxIo4Decoder;

impl ButtonDecoder for AdxIo4Decoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < DATA_OFFSET + 5 {
            return Err(format!(
                "ADX IO4 report is {} bytes; at least 31 are required",
                report.len()
            ));
        }
        let data = &report[DATA_OFFSET..DATA_OFFSET + 5];
        let mut word = ButtonWord::EMPTY;
        let active_high = [
            (data[0] & 0x01 != 0, Button::Coin),
            (data[4] & 0x02 != 0, Button::Select),
            (data[3] & 0x02 != 0, Button::Up),
            (data[3] & 0x40 != 0, Button::Down),
        ];
        for (pressed, button) in active_high {
            if pressed {
                word.set(button);
            }
        }
        let active_low = [
            (data[3], 0x08, Button::Ba1),
            (data[3], 0x04, Button::Ba2),
            (data[3], 0x01, Button::Ba3),
            (data[4], 0x80, Button::Ba4),
            (data[4], 0x40, Button::Ba5),
            (data[4], 0x20, Button::Ba6),
            (data[4], 0x10, Button::Ba7),
            (data[4], 0x08, Button::Ba8),
        ];
        for (byte, mask, button) in active_low {
            if byte & mask == 0 {
                word.set(button);
            }
        }
        Ok(word)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/adx_io4/report__tests.rs"]
mod tests;
