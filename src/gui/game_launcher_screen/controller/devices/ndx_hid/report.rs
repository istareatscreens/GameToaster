use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const NDX_HID_VID: u16 = 0x2E3C;
pub(crate) const NDX_HID_ALT_VID: u16 = 0x2E4C;
pub(crate) const DEFAULT_PID: u16 = 0x5750;

pub(crate) const REPORT_SIZE: usize = 14;

#[derive(Default)]
pub(crate) struct NdxHidDecoder;

impl ButtonDecoder for NdxHidDecoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < REPORT_SIZE {
            return Err(format!(
                "the ring's report is {} bytes; {REPORT_SIZE} are required",
                report.len()
            ));
        }
        let fields = &report[1..REPORT_SIZE];

        let order = [
            (4, Button::Ba1),
            (3, Button::Ba2),
            (2, Button::Ba3),
            (1, Button::Ba4),
            (8, Button::Ba5),
            (7, Button::Ba6),
            (6, Button::Ba7),
            (5, Button::Ba8),
            (9, Button::Select),
        ];
        let mut word = ButtonWord::EMPTY;
        for (index, button) in order {
            if fields[index] == 0x01 {
                word.set(button);
            }
        }
        Ok(word)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/ndx_hid/report__tests.rs"]
mod tests;
