use super::*;
use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

#[test]
fn identity_is_the_default_ring_and_report_size_matches_the_decoder() {
    assert_eq!(identity(), (0x2E3C, 0x5750, 13));
    assert!(decoder().decode(&[0u8; 13]).is_ok());
    assert!(decoder().decode(&[0u8; 12]).is_err());
}
