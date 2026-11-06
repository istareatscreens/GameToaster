use super::*;
use crate::game_launcher_screen::controller::framework::buttons::Button;

fn report(ring: u8, keys: u8) -> [u8; REPORT_SIZE] {
    [0xFF, 0xFF, 0xFF, 0xFF, 0x03, 0x00, ring, keys]
}

#[test]
fn each_ring_bit_is_its_button() {
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
    for (bit, button) in ring.into_iter().enumerate() {
        let word = NproDecoder.decode(&report(1 << bit, 0)).unwrap();
        assert!(word.contains(button), "bit {bit}");
        assert_eq!(word.bits(), button.mask(), "bit {bit} sets only its button");
    }
}

#[test]
fn the_touch_sensors_press_nothing() {
    assert_eq!(
        NproDecoder.decode(&report(0, 0)).unwrap(),
        ButtonWord::EMPTY
    );
}

#[test]
fn only_the_players_select_of_the_cabinet_keys_is_read() {
    assert!(NproDecoder
        .decode(&report(0, 0x01))
        .unwrap()
        .contains(Button::Select));
    assert_eq!(
        NproDecoder.decode(&report(0, 0x0E)).unwrap(),
        ButtonWord::EMPTY
    );
}
