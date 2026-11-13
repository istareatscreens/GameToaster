use super::*;

#[test]
fn a_press_during_the_button_test_only_shows_in_the_readout() {
    assert_eq!(press_duty(true, false), PressDuty::Readout);

    assert_eq!(press_duty(true, true), PressDuty::Readout);
}

#[test]
fn a_press_during_a_launch_can_only_cancel_it() {
    assert_eq!(press_duty(false, true), PressDuty::CancelLaunch);
}

#[test]
fn a_press_on_the_launcher_screen_performs_its_command() {
    assert_eq!(press_duty(false, false), PressDuty::Command);
}

#[test]
fn the_readout_names_the_held_buttons_by_their_real_numbers() {
    let mut held = ButtonWord::EMPTY;
    assert_eq!(button_labels(held), "none");
    held.set(Button::Ba1);
    held.set(Button::Ba8);
    held.set(Button::Coin);
    assert_eq!(button_labels(held), "COIN, BA1, BA8");
}
