use super::*;

#[test]
fn round_trips_through_string() {
    for text in ["Ctrl+Alt+M", "Ctrl+Shift+F5", "Alt+1", "Ctrl+Space"] {
        let hotkey: Hotkey = text.parse().unwrap();
        assert_eq!(hotkey.to_string(), text);
        assert!(hotkey.is_valid());
    }
}

#[test]
fn bare_letter_is_invalid_but_function_key_is_valid() {
    let bare = Hotkey {
        ctrl: false,
        alt: false,
        shift: false,
        win: false,
        key: 0x41,
    };
    assert!(!bare.is_valid());
    let f5: Hotkey = "F5".parse().unwrap();
    assert!(f5.is_valid());
}

#[test]
fn default_launcher_hotkey_is_ctrl_alt_m() {
    assert_eq!(default_launcher_hotkey().to_string(), "Ctrl+Alt+M");
}
