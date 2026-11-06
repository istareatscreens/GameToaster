use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const DEFAULT_VID: u16 = 0x0CA3;
pub(crate) const DEFAULT_PID: u16 = 0x0021;
pub(crate) const REPORT_SIZE: usize = 64;
pub(crate) const DATA_OFFSET: usize = 26;

#[derive(Default)]
pub(crate) struct NdxIo4Decoder;

impl ButtonDecoder for NdxIo4Decoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < DATA_OFFSET + 5 {
            return Err(format!(
                "the ring's report is {} bytes; at least {} are required",
                report.len(),
                DATA_OFFSET + 5
            ));
        }
        let data = &report[DATA_OFFSET..DATA_OFFSET + 5];
        let mut word = ButtonWord::EMPTY;

        if data[3] & 0x02 != 0 {
            word.set(Button::Select);
        }
        let active_low = [
            (data[3], 0x04, Button::Ba1),
            (data[3], 0x08, Button::Ba2),
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
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/ndx_io4/report__tests.rs"]
mod tests;
