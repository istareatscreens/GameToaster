use super::*;

fn idle() -> [u8; REPORT_SIZE] {
    let mut report = [0; REPORT_SIZE];
    report[0] = 0x01;
    for analogue in 0..8 {
        report[2 + analogue * 2] = 0x80;
    }
    report[25] = 0x02;
    report[27] = 0x02;
    report[29] = 0x0D;
    report[30] = 0xF8;
    report[31] = 0x0D;
    report[32] = 0xF8;
    report[33] = 0x30;
    report
}

fn decode(report: &[u8]) -> ButtonWord {
    Io4ReportDecoder.decode(report).unwrap()
}

#[test]
fn the_idle_report_presses_nothing() {
    assert_eq!(decode(&idle()), ButtonWord::EMPTY);
}

#[test]
fn the_ring_is_wired_as_the_stock_board_wires_it() {
    let cases = [
        (29, 0x04, Button::Ba1),
        (29, 0x08, Button::Ba2),
        (29, 0x01, Button::Ba3),
        (30, 0x80, Button::Ba4),
        (30, 0x40, Button::Ba5),
        (30, 0x20, Button::Ba6),
        (30, 0x10, Button::Ba7),
        (30, 0x08, Button::Ba8),
    ];
    for (at, mask, button) in cases {
        let mut report = idle();
        report[at] &= !mask;
        assert_eq!(decode(&report).bits(), button.mask(), "{}", button.label());
    }
}

#[test]
fn the_function_keys_are_service_test_and_the_two_selects() {
    let cases = [
        (26, 0x01, Button::Coin),
        (30, 0x02, Button::Select),
        (29, 0x02, Button::Up),
        (29, 0x40, Button::Down),
    ];
    for (at, mask, button) in cases {
        let mut report = idle();
        report[at] |= mask;
        assert_eq!(decode(&report).bits(), button.mask(), "{}", button.label());
    }
}

#[test]
fn the_second_players_word_presses_nothing_here() {
    let mut report = idle();
    report[31] = 0x00;
    report[32] = 0x00;
    assert_eq!(decode(&report), ButtonWord::EMPTY);
}

#[test]
fn a_ring_in_hid_mode_is_refused_instead_of_read_as_eight_held_buttons() {
    let mut report = [0u8; 33];
    report[1] = 0x02;
    let error = Io4ReportDecoder.decode(&report).unwrap_err();
    assert!(error.contains("report id 0x00"), "{error}");
    assert!(error.contains("33 bytes"), "{error}");

    assert!(Io4ReportDecoder.decode(&[0u8; REPORT_SIZE]).is_err());
}

#[test]
fn a_report_too_short_for_the_button_word_is_refused() {
    let report = idle();
    assert!(Io4ReportDecoder.decode(&report[..30]).is_err());
    assert!(Io4ReportDecoder.decode(&report[..31]).is_ok());
    assert!(Io4ReportDecoder.decode(&[]).is_err());
}
