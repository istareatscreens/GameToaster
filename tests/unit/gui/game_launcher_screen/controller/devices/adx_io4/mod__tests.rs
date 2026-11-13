use super::*;

#[test]
fn the_driver_reads_the_stock_boards_report_at_its_identity() {
    assert_eq!(identity(), (0x0CA3, 0x0021, 64));
    assert_eq!(IDENTITIES, &[(0x0CA3, 0x0021)]);
    assert_eq!(DESCRIPTOR.id, "adx-io4");
}
