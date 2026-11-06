use super::*;

#[test]
fn the_press_that_starts_a_launch_can_never_cancel_it() {
    let mut arming = CancelArming::default();
    arming.begin();
    assert!(
        !arming.may_cancel(),
        "a press arriving while the button is still held from the launch must do nothing"
    );
}

#[test]
fn contact_bounce_from_the_launching_press_does_not_arm_the_cancel() {
    for bounce in [
        Duration::ZERO,
        Duration::from_millis(1),
        Duration::from_millis(8),
        Duration::from_millis(50),
        ARM_AFTER - Duration::from_millis(1),
    ] {
        let mut arming = CancelArming::default();
        arming.begin();
        arming.released(bounce);
        assert!(
            !arming.may_cancel(),
            "a release {bounce:?} after the launch is bounce, not a deliberate release"
        );
    }
}

#[test]
fn a_real_release_after_the_guard_window_arms_the_cancel() {
    for held in [
        ARM_AFTER,
        ARM_AFTER + Duration::from_millis(1),
        Duration::from_secs(5),
    ] {
        let mut arming = CancelArming::default();
        arming.begin();
        arming.released(held);
        assert!(
            arming.may_cancel(),
            "a release {held:?} after the launch is the operator letting go"
        );
    }
}

#[test]
fn a_second_launch_disarms_again_so_each_launch_needs_its_own_release() {
    let mut arming = CancelArming::default();
    arming.begin();
    arming.released(Duration::from_secs(1));
    assert!(arming.may_cancel());

    arming.begin();
    assert!(
        !arming.may_cancel(),
        "the new launch must not inherit the previous launch's arming"
    );
}

#[test]
fn bounce_after_a_real_release_cannot_disarm_what_is_already_armed() {
    let mut arming = CancelArming::default();
    arming.begin();
    arming.released(Duration::from_secs(1));
    arming.released(Duration::from_millis(2));
    assert!(
        arming.may_cancel(),
        "once the operator has let go, the cancel stays available"
    );
}

#[test]
fn the_guard_window_is_far_longer_than_bounce_and_far_shorter_than_a_deliberate_press() {
    assert!(
        ARM_AFTER >= Duration::from_millis(100),
        "the window must cover contact bounce and the rings' poll interval"
    );
    assert!(
        ARM_AFTER <= Duration::from_millis(1000),
        "the window must not make a deliberate cancel feel broken"
    );
}
