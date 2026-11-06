use super::*;
use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

#[test]
fn both_rings_and_the_report_size_match_the_decoder() {
    assert_eq!(IDENTITIES, &[(0x2E3C, 0x5751), (0x2E3C, 0x5752)]);
    assert!(decoder().decode(&[0u8; REPORT_SIZE]).is_ok());
    assert!(decoder().decode(&[0u8; REPORT_SIZE - 1]).is_err());
}

#[test]
fn it_declares_no_lights_because_its_lighting_is_on_its_own_usb_connection() {
    assert!(DESCRIPTOR.lights.is_none());
}

#[test]
fn it_opens_the_rings_own_interface_class_on_interface_zero() {
    assert_eq!(
        format!("{INTERFACE_CLASS:?}"),
        "7F2A9C41-5E3B-4D8A-9F16-3C7E5B2D9A48"
    );
    assert_eq!(INTERFACE_NUMBER, 0);
}
