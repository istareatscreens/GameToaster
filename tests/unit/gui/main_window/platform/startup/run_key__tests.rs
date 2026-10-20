use super::*;

#[test]
fn the_tray_flag_marks_a_sign_in_start_in_any_letter_case() {
    let args = |values: &[&str]| -> Vec<OsString> { values.iter().map(OsString::from).collect() };
    assert!(launched_at_sign_in(&args(&["--tray"])));
    assert!(launched_at_sign_in(&args(&["--other", "--TRAY"])));
    assert!(!launched_at_sign_in(&args(&[])));
    assert!(!launched_at_sign_in(&args(&["--trayicon", "tray"])));
}
