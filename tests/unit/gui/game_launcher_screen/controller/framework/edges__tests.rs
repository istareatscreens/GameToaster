use super::super::buttons::Button;
use super::*;

#[test]
fn held_button_generates_one_rising_edge() {
    let mut tracker = EdgeTracker::default();
    let pressed = ButtonWord::from_bits(Button::Ba6.mask());
    assert!(tracker.update(pressed).rising.contains(Button::Ba6));
    assert_eq!(tracker.update(pressed).rising, ButtonWord::EMPTY);
    assert!(tracker
        .update(ButtonWord::EMPTY)
        .falling
        .contains(Button::Ba6));
    assert!(tracker.update(pressed).rising.contains(Button::Ba6));
}

#[test]
fn reconnect_requires_held_buttons_to_release() {
    let mut tracker = EdgeTracker::default();
    let pressed = ButtonWord::from_bits(Button::Ba4.mask());
    tracker.establish_baseline(pressed);
    assert_eq!(tracker.update(pressed).rising, ButtonWord::EMPTY);
    tracker.update(ButtonWord::EMPTY);
    assert!(tracker.update(pressed).rising.contains(Button::Ba4));
    assert!(tracker.disconnect().falling.contains(Button::Ba4));
}
