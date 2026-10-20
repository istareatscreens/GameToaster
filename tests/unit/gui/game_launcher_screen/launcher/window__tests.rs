use super::*;

#[test]
fn frame_visibility_hides_but_never_skips() {
    use FrameVisibility::{Hidden, Launching, Menu};
    assert_eq!(frame_visibility(true, false, false, false), Menu);
    assert_eq!(frame_visibility(true, true, false, false), Hidden);
    assert_eq!(frame_visibility(false, false, true, false), Launching);
    assert_eq!(frame_visibility(true, true, true, false), Launching);
    assert_eq!(frame_visibility(true, false, true, false), Menu);

    assert_eq!(frame_visibility(true, false, false, true), Hidden);
    assert_eq!(frame_visibility(false, false, false, false), Hidden);
}
