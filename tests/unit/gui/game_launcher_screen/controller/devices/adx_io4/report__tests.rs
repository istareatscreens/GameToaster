use super::*;

fn idle() -> [u8; REPORT_SIZE] {
    let mut report = [0; REPORT_SIZE];
    report[29] = 0x0D;
    report[30] = 0xF8;
    report
}

#[test]
fn idle_and_every_ba_bit_match_the_c_decoder() {
    let decoder = AdxIo4Decoder;
    assert_eq!(decoder.decode(&idle()).unwrap(), ButtonWord::EMPTY);
    let cases = [
        (29, 0x08, Button::Ba1),
        (29, 0x04, Button::Ba2),
        (29, 0x01, Button::Ba3),
        (30, 0x80, Button::Ba4),
        (30, 0x40, Button::Ba5),
        (30, 0x20, Button::Ba6),
        (30, 0x10, Button::Ba7),
        (30, 0x08, Button::Ba8),
    ];
    for (offset, mask, button) in cases {
        let mut report = idle();
        report[offset] &= !mask;
        assert_eq!(decoder.decode(&report).unwrap().bits(), button.mask());
    }
}

#[test]
fn short_report_is_rejected() {
    assert!(AdxIo4Decoder.decode(&[0; 30]).is_err());
}
