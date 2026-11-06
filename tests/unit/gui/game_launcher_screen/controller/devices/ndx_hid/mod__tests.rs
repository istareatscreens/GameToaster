use super::*;
use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

#[test]
fn identity_is_the_newer_ring_and_report_size_matches_the_decoder() {
    assert_eq!(identity(), (0x2E3C, 0x5750, 14));
    assert!(decoder().decode(&[0u8; 14]).is_ok());
    assert!(decoder().decode(&[0u8; 13]).is_err());
}

#[test]
fn a_second_ring_is_reached_at_its_own_vendor_id() {
    assert_eq!(IDENTITIES, &[(0x2E3C, 0x5750), (0x2E4C, 0x5750)]);
}
