use super::*;

#[test]
fn all_twelve_fields_match_the_verified_order() {
    let decoder = AdxHidDecoder;
    let order = [
        Button::Ba4,
        Button::Ba3,
        Button::Ba2,
        Button::Ba1,
        Button::Ba8,
        Button::Ba7,
        Button::Ba6,
        Button::Ba5,
        Button::Up,
        Button::Select,
        Button::Down,
        Button::Coin,
    ];
    for (index, expected) in order.iter().enumerate() {
        let mut report = [0; REPORT_SIZE];
        report[index + 1] = 1;
        assert_eq!(decoder.decode(&report).unwrap().bits(), expected.mask());
    }
    assert!(decoder.decode(&[0; 12]).is_err());
}
