use crate::gui::game_launcher_screen::controller::framework::adapter::ButtonDecoder;
use crate::gui::game_launcher_screen::controller::framework::buttons::{Button, ButtonWord};

pub(crate) const VID: u16 = 0x0CA3;
pub(crate) const PID: u16 = 0x0021;
pub(crate) const REPORT_SIZE: usize = 64;
const REPORT_ID: u8 = 0x01;

const MIN_LEN: usize = 31;

#[derive(Default)]
pub(crate) struct Io4ReportDecoder;

impl ButtonDecoder for Io4ReportDecoder {
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String> {
        if report.len() < MIN_LEN {
            return Err(format!(
                "the I/O board report is {} bytes; at least {MIN_LEN} are required",
                report.len()
            ));
        }
        if report[0] != REPORT_ID {
            return Err(format!(
                "not an I/O board report (report id 0x{:02X}, {} bytes); a ring in HID mode needs a HID controller type",
                report[0],
                report.len()
            ));
        }
        let mut word = ButtonWord::EMPTY;
        let active_low = [
            (29, 0x04, Button::Ba1),
            (29, 0x08, Button::Ba2),
            (29, 0x01, Button::Ba3),
            (30, 0x80, Button::Ba4),
            (30, 0x40, Button::Ba5),
            (30, 0x20, Button::Ba6),
            (30, 0x10, Button::Ba7),
            (30, 0x08, Button::Ba8),
        ];
        for (at, mask, button) in active_low {
            if report[at] & mask == 0 {
                word.set(button);
            }
        }
        let active_high = [
            (26, 0x01, Button::Coin),
            (30, 0x02, Button::Select),
            (29, 0x02, Button::Up),
            (29, 0x40, Button::Down),
        ];
        for (at, mask, button) in active_high {
            if report[at] & mask != 0 {
                word.set(button);
            }
        }
        Ok(word)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/devices/io4_report__tests.rs"]
mod tests;
