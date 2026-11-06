use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const DEFAULT_VID: u16 = 0x0E8F;
pub(crate) const DEFAULT_PID: u16 = 0x1224;

pub(crate) const REPORT_SIZE: usize = 8;

#[derive(Default)]
pub(crate) struct DaoDecoder;

impl ButtonDecoder for DaoDecoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < REPORT_SIZE {
            return Err(format!(
                "the controller's report is {} bytes; {REPORT_SIZE} are required",
                report.len()
            ));
        }
        let data = &report[1..REPORT_SIZE];
        let ring = [
            Button::Ba1,
            Button::Ba2,
            Button::Ba3,
            Button::Ba4,
            Button::Ba5,
            Button::Ba6,
            Button::Ba7,
            Button::Ba8,
        ];
        let mut word = ButtonWord::EMPTY;
        for (bit, button) in ring.into_iter().enumerate() {
            if data[5] & (1 << bit) != 0 {
                word.set(button);
            }
        }

        if data[6] & 0x08 != 0 {
            word.set(Button::Select);
        }
        Ok(word)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/dao/report__tests.rs"]
mod tests;
