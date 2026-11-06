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
fn the_player_select_is_its_own_bit_and_the_cabinet_keys_drive_nothing() {
    let mut raw = vec![0u8; REPORT_SIZE];
    raw[1 + 6] = 0x08;
    assert_eq!(
        pressed(DaoDecoder.decode(&raw).unwrap()),
        vec![Button::Select]
    );
    for mask in [0x01, 0x02, 0x04] {
        let mut raw = vec![0u8; REPORT_SIZE];
        raw[1 + 6] = mask;
        assert_eq!(
            DaoDecoder.decode(&raw).unwrap(),
            ButtonWord::EMPTY,
            "mask {mask:#x}"
        );
    }
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
