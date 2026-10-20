use super::*;

#[test]
fn every_byte_becomes_two_lower_case_digits() {
    assert_eq!(encode(&[]), "");
    assert_eq!(encode(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
}
