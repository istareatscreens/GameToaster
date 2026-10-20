use super::*;

#[test]
fn embedded_toaster_icon_decodes() {
    let missing = std::env::temp_dir().join("gametoaster-no-external-icon");
    let (rgba, width, height) = icon_rgba(&missing, 32).unwrap();
    assert_eq!((width, height, rgba.len()), (32, 32, 32 * 32 * 4));
}
