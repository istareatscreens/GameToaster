use super::*;

#[test]
fn the_word_mask_is_derived_from_every_button() {
    let union = Button::ALL
        .into_iter()
        .fold(0u16, |mask, button| mask | button.mask());
    assert_eq!(ButtonWord::from_bits(u16::MAX).bits(), union);

    assert_eq!(union, 0x0FFF);
}

#[test]
fn every_button_has_its_hardware_label() {
    let labels: Vec<&str> = Button::ALL.into_iter().map(Button::label).collect();
    assert_eq!(
        labels,
        vec![
            "COIN", "BA1", "BA2", "BA3", "BA4", "BA5", "BA6", "BA7", "BA8", "SELECT", "UP", "DOWN"
        ]
    );
}
