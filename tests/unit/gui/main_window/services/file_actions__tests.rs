use super::*;

#[test]
fn selection_validation_rejects_relative_missing_and_non_launchable_paths() {
    assert!(validate_selected_executable(Path::new("relative.exe")).is_err());
    assert!(validate_selected_executable(Path::new(r"C:\missing.exe")).is_err());
    assert!(validate_selected_executable(Path::new(r"C:\missing.txt")).is_err());
}
