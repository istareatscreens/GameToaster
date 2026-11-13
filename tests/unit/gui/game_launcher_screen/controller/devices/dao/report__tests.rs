use super::*;

fn pressed(word: ButtonWord) -> Vec<Button> {
    Button::ALL
        .into_iter()
        .filter(|button| word.contains(*button))
        .collect()
}

#[test]
fn each_ring_button_is_one_bit_of_its_own_byte() {
    for (bit, button) in [
        (0, Button::Ba1),
        (1, Button::Ba2),
        (2, Button::Ba3),
        (3, Button::Ba4),
        (4, Button::Ba5),
        (5, Button::Ba6),
        (6, Button::Ba7),
        (7, Button::Ba8),
    ] {
        let mut raw = vec![0u8; REPORT_SIZE];
        raw[1 + 5] = 1 << bit;
        assert_eq!(pressed(DaoDecoder.decode(&raw).unwrap()), vec![button]);
    }
}

#[test]
fn the_function_keys_are_named_as_on_every_ring() {
    for (mask, button) in [
        (0x01, Button::Coin),
        (0x02, Button::Down),
        (0x04, Button::Select),
        (0x08, Button::Up),
    ] {
        let mut raw = vec![0u8; REPORT_SIZE];
        raw[1 + 6] = mask;
        assert_eq!(
            pressed(DaoDecoder.decode(&raw).unwrap()),
            vec![button],
            "mask {mask:#x}"
        );
    }
    let mut raw = vec![0u8; REPORT_SIZE];
    raw[1 + 6] = 0xF0;
    assert_eq!(DaoDecoder.decode(&raw).unwrap(), ButtonWord::EMPTY);
}

#[test]
fn the_reference_report_presses_its_buttons_and_keys() {
    let raw = [0x00, 0x01, 0x80, 0x03, 0x02, 0x80, 0x05, 0x0D];
    assert_eq!(
        pressed(DaoDecoder.decode(&raw).unwrap()),
        vec![
            Button::Coin,
            Button::Ba1,
            Button::Ba3,
            Button::Select,
            Button::Up
        ]
    );
}

#[test]
fn the_touch_sensors_in_the_same_report_press_nothing() {
    let mut raw = vec![0u8; REPORT_SIZE];
    for index in 0..5 {
        raw[1 + index] = 0xFF;
    }
    assert_eq!(DaoDecoder.decode(&raw).unwrap(), ButtonWord::EMPTY);
}

#[test]
fn nothing_held_is_nothing_pressed_and_a_short_report_is_an_error() {
    assert_eq!(
        DaoDecoder.decode(&[0u8; REPORT_SIZE]).unwrap(),
        ButtonWord::EMPTY
    );
    assert!(DaoDecoder.decode(&[0u8; REPORT_SIZE - 1]).is_err());
}
