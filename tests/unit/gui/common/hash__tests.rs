use super::*;

#[test]
fn empty_is_the_offset_basis_and_content_changes_the_hash() {
    assert_eq!(fnv1a_64(&[]), 0xcbf29ce484222325);
    assert_ne!(fnv1a_64(b"a"), fnv1a_64(b"b"));

    assert_eq!(fnv1a_64(b"a"), 0xaf63dc4c8601ec8c);
}
