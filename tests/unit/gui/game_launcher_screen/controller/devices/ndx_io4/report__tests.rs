use super::*;

fn idle() -> Vec<u8> {
    let mut raw = vec![0u8; REPORT_SIZE];
    raw[DATA_OFFSET + 1] = 0x02;
    raw[DATA_OFFSET + 3] = 0x0D;
    raw[DATA_OFFSET + 4] = 0xF8;
    raw
}

fn pressed(word: ButtonWord) -> Vec<Button> {
    Button::ALL
        .into_iter()
        .filter(|button| word.contains(*button))
        .collect()
}

#[test]
fn an_idle_report_presses_nothing() {
    assert_eq!(NdxIo4Decoder.decode(&idle()).unwrap(), ButtonWord::EMPTY);
}

#[test]
fn each_ring_button_decodes_from_its_own_active_low_bit() {
    for (byte, mask, button) in [
        (3, 0x04, Button::Ba1),
        (3, 0x08, Button::Ba2),
        (3, 0x01, Button::Ba3),
        (4, 0x80, Button::Ba4),
        (4, 0x40, Button::Ba5),
        (4, 0x20, Button::Ba6),
        (4, 0x10, Button::Ba7),
        (4, 0x08, Button::Ba8),
    ] {
        let mut raw = idle();
        raw[DATA_OFFSET + byte] &= !mask;
        assert_eq!(pressed(NdxIo4Decoder.decode(&raw).unwrap()), vec![button]);
    }
}

#[test]
fn the_player_select_is_the_bit_the_older_board_reads_as_up() {
    let mut raw = idle();
    raw[DATA_OFFSET + 3] |= 0x02;
    assert_eq!(
        pressed(NdxIo4Decoder.decode(&raw).unwrap()),
        vec![Button::Select]
    );

    let older = crate::gui::game_launcher_screen::controller::devices::adx_io4::decoder()
        .decode(&raw)
        .unwrap();
    assert!(older.contains(Button::Up) && !older.contains(Button::Select));
}

#[test]
fn ba1_and_ba2_are_the_other_way_round_from_the_older_board() {
    let mut raw = idle();
    raw[DATA_OFFSET + 3] &= !0x04;
    let newer = NdxIo4Decoder.decode(&raw).unwrap();
    let older = crate::gui::game_launcher_screen::controller::devices::adx_io4::decoder()
        .decode(&raw)
        .unwrap();
    assert!(newer.contains(Button::Ba1) && !newer.contains(Button::Ba2));
    assert!(older.contains(Button::Ba2) && !older.contains(Button::Ba1));
}

#[test]
fn the_cabinet_keys_drive_nothing_on_the_launcher_screen() {
    for (byte, mask) in [(4, 0x02), (3, 0x40), (0, 0x01)] {
        let mut raw = idle();
        raw[DATA_OFFSET + byte] |= mask;
        let word = NdxIo4Decoder.decode(&raw).unwrap();
        assert_eq!(word, ButtonWord::EMPTY, "byte {byte} mask {mask:#x}");
    }
}

#[test]
fn a_report_too_short_for_the_button_field_is_an_error() {
    assert!(NdxIo4Decoder.decode(&[0u8; DATA_OFFSET + 4]).is_err());
}
