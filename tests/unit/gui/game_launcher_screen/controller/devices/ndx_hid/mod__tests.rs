use super::*;

#[test]
fn the_driver_reads_the_rings_hid_report_at_both_ring_identities() {
    assert_eq!(identity(), (0x2E3C, 0x5750, 14));
    assert_eq!(IDENTITIES, &[(0x2E3C, 0x5750), (0x2E4C, 0x5750)]);
    assert_eq!(DESCRIPTOR.id, "ndx-hid");
}
