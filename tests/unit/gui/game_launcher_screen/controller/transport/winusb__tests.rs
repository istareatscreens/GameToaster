use super::*;

#[test]
fn reports_split_and_joined_across_reads_come_out_whole() {
    let mut framer = ReportFramer::new(4);
    let mut seen: Vec<Vec<u8>> = Vec::new();
    framer.push(&[1, 2], |report| seen.push(report.to_vec()));
    assert!(seen.is_empty());
    framer.push(&[3, 4, 5, 6, 7, 8, 9], |report| seen.push(report.to_vec()));
    assert_eq!(seen, vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]]);
    framer.push(&[10, 11, 12], |report| seen.push(report.to_vec()));
    assert_eq!(seen.last().unwrap(), &vec![9, 10, 11, 12]);
}

#[test]
fn a_long_read_never_overruns_the_report() {
    let mut framer = ReportFramer::new(8);
    let mut count = 0;
    framer.push(&[0u8; 64], |report| {
        assert_eq!(report.len(), 8);
        count += 1;
    });
    assert_eq!(count, 8);
}

#[test]
fn the_interface_number_comes_from_the_hardware_id() {
    let composite = vec![r"USB\VID_3356&PID_3003&REV_0100&MI_01".to_string()];
    assert_eq!(node_interface(&composite, 0x3356, 0x3003), Some(1));
    let single = vec![r"USB\VID_2E3C&PID_5751&REV_0100".to_string()];
    assert_eq!(node_interface(&single, 0x2E3C, 0x5751), Some(0));
    assert_eq!(node_interface(&single, 0x2E3C, 0x5752), None);
}

#[test]
fn the_registered_interface_classes_are_read_with_or_without_braces() {
    let guids =
        parse_guids("{7F2A9C41-5E3B-4D8A-9F16-3C7E5B2D9A48}\0A5DCBF10-6530-11D2-901F-00C04FB951ED");
    assert_eq!(guids.len(), 2);
    assert_eq!(
        guids[0],
        GUID::from_u128(0x7F2A9C41_5E3B_4D8A_9F16_3C7E5B2D9A48)
    );
}
