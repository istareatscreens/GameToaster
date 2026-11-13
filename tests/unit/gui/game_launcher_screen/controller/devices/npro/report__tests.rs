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
fn the_function_keys_are_named_as_on_every_ring() {
    for (mask, button) in [
        (0x01, Button::Up),
        (0x02, Button::Coin),
        (0x04, Button::Down),
        (0x08, Button::Select),
    ] {
        assert_eq!(
            NproDecoder.decode(&report(0, mask)).unwrap().bits(),
            button.mask(),
            "mask {mask:#x}"
        );
    }
    assert_eq!(
        NproDecoder.decode(&report(0, 0xF0)).unwrap(),
        ButtonWord::EMPTY
    );
}

#[test]
fn the_reference_report_presses_its_buttons_and_keys() {
    let raw = [0x01, 0x00, 0x01, 0x00, 0x06, 0x80, 0x81, 0x0B];
    assert_eq!(
        NproDecoder.decode(&raw).unwrap().bits(),
        Button::Ba1.mask()
            | Button::Ba8.mask()
            | Button::Up.mask()
            | Button::Coin.mask()
            | Button::Select.mask()
    );
}
