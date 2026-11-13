use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const VID: u16 = 0x2E3C;
pub(crate) const ALT_VID: u16 = 0x2E4C;
pub(crate) const PID: u16 = 0x5750;

pub(crate) const REPORT_SIZE: usize = 14;
const FIRST_FIELD: usize = 2;
const PRESSED: u8 = 0x01;

#[derive(Default)]
pub(crate) struct YuanHidReportDecoder;

impl ButtonDecoder for YuanHidReportDecoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < REPORT_SIZE {
            return Err(format!(
                "the ring's report is {} bytes; {REPORT_SIZE} are required",
                report.len()
            ));
        }
        let fields = [
            Button::Ba4,
            Button::Ba3,
            Button::Ba2,
            Button::Ba1,
            Button::Ba8,
            Button::Ba7,
            Button::Ba6,
            Button::Ba5,
            Button::Up,
            Button::Select,
            Button::Down,
            Button::Coin,
        ];
        let mut word = ButtonWord::EMPTY;
        for (value, button) in report[FIRST_FIELD..REPORT_SIZE].iter().zip(fields) {
            if *value == PRESSED {
                word.set(button);
            }
        }
        Ok(word)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/devices/yuan_hid_report__tests.rs"]
mod tests;
