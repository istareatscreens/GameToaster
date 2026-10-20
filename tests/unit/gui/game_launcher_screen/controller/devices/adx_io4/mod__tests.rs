use super::*;
use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

#[test]
fn factory_identity_and_decoder_are_composable() {
    assert_eq!(identity(), (0x0CA3, 0x0021, 64));
    let report = [0u8; 64];
    assert!(decoder().decode(&report).is_ok());
}
