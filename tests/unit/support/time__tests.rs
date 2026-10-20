use super::*;

#[test]
fn utc_format_is_exact() {
    let time = UNIX_EPOCH + std::time::Duration::from_millis(1_704_067_200_123);
    assert_eq!(utc_stamp(time).unwrap(), "20240101T000000.123Z");
}
