use std::path::Path;

use super::*;

#[test]
fn a_wide_string_is_utf16_with_one_trailing_null() {
    assert_eq!(wide("ab"), [0x61, 0x62, 0]);
    assert_eq!(wide(Path::new(r"C:\é")), [0x43, 0x3a, 0x5c, 0xe9, 0]);
    assert_eq!(wide(OsStr::new("")), [0]);
}
