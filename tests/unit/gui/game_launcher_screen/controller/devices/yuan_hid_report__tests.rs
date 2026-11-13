use super::*;

fn idle() -> [u8; 33] {
    let mut report = [0; 33];
    report[1] = 0x02;
    report[32] = 0x80;
    report
}

fn decode(report: &[u8]) -> ButtonWord {
    YuanHidReportDecoder.decode(report).unwrap()
}

#[test]
fn the_idle_report_presses_nothing() {
    assert_eq!(decode(&idle()), ButtonWord::EMPTY);
}

#[test]
fn every_field_is_its_key() {
    let cases = [
        (2, Button::Ba4),
        (3, Button::Ba3),
        (4, Button::Ba2),
        (5, Button::Ba1),
        (6, Button::Ba8),
        (7, Button::Ba7),
        (8, Button::Ba6),
        (9, Button::Ba5),
        (10, Button::Up),
        (11, Button::Select),
        (12, Button::Down),
        (13, Button::Coin),
    ];
    for (at, button) in cases {
        let mut report = idle();
        report[at] = 0x01;
        assert_eq!(decode(&report).bits(), button.mask(), "byte {at}");
    }
}

#[test]
fn pressing_the_ring_in_order_shows_buttons_one_to_eight() {
    let pressed_fields = [5, 4, 3, 2, 9, 8, 7, 6];
    let expected = [
        Button::Ba1,
        Button::Ba2,
        Button::Ba3,
        Button::Ba4,
        Button::Ba5,
        Button::Ba6,
        Button::Ba7,
        Button::Ba8,
    ];
    for (at, button) in pressed_fields.into_iter().zip(expected) {
        let mut report = idle();
        report[at] = 0x01;
        assert_eq!(decode(&report).bits(), button.mask(), "byte {at}");
    }
}

#[test]
fn the_constant_before_the_fields_is_not_a_key() {
    let mut report = idle();
    report[1] = 0x01;
    assert_eq!(decode(&report), ButtonWord::EMPTY);
}

#[test]
fn a_field_is_pressed_only_while_it_reads_one() {
    let mut report = idle();
    report[5] = 0x02;
    report[6] = 0xFF;
    report[7] = 0x80;
    assert_eq!(decode(&report), ButtonWord::EMPTY);
}

#[test]
fn bytes_after_the_fields_are_not_read() {
    let mut report = idle();
    for byte in &mut report[14..] {
        *byte = 0x01;
    }
    assert_eq!(decode(&report), ButtonWord::EMPTY);
}

#[test]
fn held_keys_combine() {
    let mut report = idle();
    report[5] = 0x01;
    report[9] = 0x01;
    report[13] = 0x01;
    assert_eq!(
        decode(&report).bits(),
        Button::Ba1.mask() | Button::Ba5.mask() | Button::Coin.mask()
    );
}

#[test]
fn a_report_without_every_field_is_refused() {
    assert!(YuanHidReportDecoder.decode(&[0u8; 13]).is_err());
    assert!(YuanHidReportDecoder.decode(&[0u8; 14]).is_ok());
}
