use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const NPRO_VID: u16 = 0x2E3C;

pub(crate) const NPRO_PID: u16 = 0x5751;
pub(crate) const NPRO_PID_P2: u16 = 0x5752;
pub(crate) const REPORT_SIZE: usize = 8;

#[derive(Default)]
pub(crate) struct NproDecoder;

impl ButtonDecoder for NproDecoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < REPORT_SIZE {
            return Err(format!(
                "the ring's report is {} bytes; {REPORT_SIZE} are required",
                report.len()
            ));
        }
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
            if report[6] & (1 << bit) != 0 {
                word.set(button);
            }
        }
        for (mask, button) in [
            (0x01, Button::Up),
            (0x02, Button::Coin),
            (0x04, Button::Down),
            (0x08, Button::Select),
        ] {
            if report[7] & mask != 0 {
                word.set(button);
            }
        }
        Ok(word)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/npro/report__tests.rs"]
mod tests;
