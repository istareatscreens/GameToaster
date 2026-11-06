use super::*;
use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

#[test]
fn identity_and_report_size_match_the_decoder() {
    assert_eq!(identity(), (0x0E8F, 0x1224, 8));
    assert!(decoder().decode(&[0u8; 8]).is_ok());
    assert!(decoder().decode(&[0u8; 7]).is_err());
}

#[test]
fn it_declares_no_lights_because_its_lighting_is_not_the_serial_board() {
    assert!(DESCRIPTOR.lights.is_none());
}
