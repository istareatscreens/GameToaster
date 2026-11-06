use super::*;
use crate::game_launcher_screen::controller::framework::adapter::ButtonDecoder;

#[test]
fn identity_is_the_io4_board_and_report_size_matches_the_decoder() {
    assert_eq!(identity(), (0x0CA3, 0x0021, 64));
    assert!(decoder().decode(&[0u8; 31]).is_ok());
    assert!(decoder().decode(&[0u8; 30]).is_err());
}

#[test]
fn it_shares_the_older_boards_identity_so_the_configured_type_decides() {
    assert_eq!(IDENTITIES, &[(0x0CA3, 0x0021)]);
}
