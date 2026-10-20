use super::super::super::devices::{adx_hid, adx_io4};

#[test]
fn driver_owned_matching_ignores_unknown_devices() {
    let adapter = adx_io4::adapter();
    let found = adapter.match_present(&[(0x0CA3, 0x0021, 64), (0x046D, 0xC077, 8)]);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].config.driver_id, "adx-io4");
    assert_eq!(found[0].config.options.get("vid").unwrap(), "0CA3");
    assert_eq!(found[0].config.options.get("pid").unwrap(), "0021");
}

#[test]
fn alternate_adx_hid_identity_returns_complete_generic_config() {
    let adapter = adx_hid::adapter();
    let found = adapter.match_present(&[(0x2E4C, 0x5750, 64)]);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].config.driver_id, "adx-hid");
    assert_eq!(found[0].config.options.get("vid").unwrap(), "2E4C");
    assert_eq!(found[0].config.options.get("pid").unwrap(), "5750");
}

#[test]
fn empty_when_no_registered_identity_is_present() {
    assert!(adx_io4::adapter()
        .match_present(&[(0x046D, 0xC077, 8)])
        .is_empty());
    assert!(adx_hid::adapter().match_present(&[]).is_empty());
}
