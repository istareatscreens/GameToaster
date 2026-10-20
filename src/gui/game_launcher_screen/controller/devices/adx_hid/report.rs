use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const ADX_HID_VID: u16 = 0x2E3C;
pub(crate) const ADX_HID_ALT_VID: u16 = 0x2E4C;
pub(crate) const DEFAULT_PID: u16 = 0x5750;
pub(crate) const REPORT_SIZE: usize = 13;

#[derive(Default)]
pub(crate) struct AdxHidDecoder;

impl ButtonDecoder for AdxHidDecoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < REPORT_SIZE {
            return Err(format!(
                "ADX HID report is {} bytes; 13 are required",
                report.len()
            ));
        }
        let buttons = &report[1..13];
        let order = [
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
        for (value, button) in buttons.iter().zip(order) {
            if *value == 0x01 {
                word.set(button);
            }
        }
        Ok(word)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/adx_hid/report__tests.rs"]
mod tests;
