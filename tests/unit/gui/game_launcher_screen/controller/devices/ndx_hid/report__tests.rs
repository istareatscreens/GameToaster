use super::*;

fn report(length: usize) -> Vec<u8> {
    vec![0u8; length]
}

#[test]
fn each_button_decodes_from_its_own_field_one_byte_later_than_the_older_ring() {
    for (index, button) in [
        (4, Button::Ba1),
        (3, Button::Ba2),
        (2, Button::Ba3),
        (1, Button::Ba4),
        (8, Button::Ba5),
        (7, Button::Ba6),
        (6, Button::Ba7),
        (5, Button::Ba8),
        (9, Button::Select),
    ] {
        let mut raw = report(REPORT_SIZE);
        raw[1 + index] = 0x01;
        let word = NdxHidDecoder.decode(&raw).unwrap();
        let pressed: Vec<Button> = Button::ALL
            .into_iter()
            .filter(|button| word.contains(*button))
            .collect();
        assert_eq!(pressed, vec![button], "field {index}");
    }
}

#[test]
fn a_longer_report_than_the_fields_read_still_decodes() {
    let mut raw = report(33);
    raw[1 + 4] = 0x01;
    raw[1 + 9] = 0x01;
    let word = NdxHidDecoder.decode(&raw).unwrap();
    assert!(word.contains(Button::Ba1));
    assert!(word.contains(Button::Select));
    assert!(!word.contains(Button::Ba2));
}

#[test]
fn nothing_held_is_nothing_pressed_and_a_value_that_is_not_one_is_not_a_press() {
    let raw = report(REPORT_SIZE);
    assert_eq!(NdxHidDecoder.decode(&raw).unwrap(), ButtonWord::EMPTY);
    let mut noisy = report(REPORT_SIZE);
    noisy[1 + 4] = 0xFF;
    assert_eq!(NdxHidDecoder.decode(&noisy).unwrap(), ButtonWord::EMPTY);
}

#[test]
fn the_cabinet_keys_the_ring_reports_drive_nothing_on_the_launcher_screen() {
    for index in [10, 11, 12] {
        let mut raw = report(REPORT_SIZE);
        raw[1 + index] = 0x01;
        assert_eq!(
            NdxHidDecoder.decode(&raw).unwrap(),
            ButtonWord::EMPTY,
            "field {index}"
        );
    }
}

#[test]
fn a_report_too_short_for_the_fields_is_an_error_not_a_misread() {
    let raw = report(REPORT_SIZE - 1);
    assert!(NdxHidDecoder.decode(&raw).is_err());
}

#[test]
fn the_older_rings_layout_would_decode_differently_which_is_why_this_driver_exists() {
    let mut raw = report(33);
    raw[1 + 4] = 0x01;
    let ndx = NdxHidDecoder.decode(&raw).unwrap();
    let older = crate::gui::game_launcher_screen::controller::devices::adx_hid::decoder()
        .decode(&raw)
        .unwrap();
    assert!(ndx.contains(Button::Ba1));
    assert!(older.contains(Button::Ba8) && !older.contains(Button::Ba1));
}
