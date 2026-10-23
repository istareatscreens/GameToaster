use super::*;

fn denied() -> io::Error {
    io::Error::from(io::ErrorKind::PermissionDenied)
}

#[test]
fn a_portable_install_in_an_unwritable_folder_is_explained_with_its_folder() {
    let _locale = crate::gui::common::harness::locale("en");
    let text = state_folder_message(InstallKind::Portable, Path::new("some-folder"), &denied())
        .expect("a portable install and a denied write are explained");
    assert!(text.contains("some-folder"), "{text}");
    assert!(text.contains("beside GameToaster.exe"), "{text}");
    assert!(!text.contains("os error"), "{text}");
}

#[test]
fn another_error_or_a_packaged_install_keeps_the_technical_text() {
    let _locale = crate::gui::common::harness::locale("en");
    let folder = Path::new("some-folder");
    assert_eq!(
        state_folder_message(
            InstallKind::Portable,
            folder,
            &io::Error::from(io::ErrorKind::StorageFull)
        ),
        None
    );
    assert_eq!(
        state_folder_message(
            InstallKind::Portable,
            folder,
            &io::Error::from(io::ErrorKind::NotFound)
        ),
        None
    );
    assert_eq!(
        state_folder_message(InstallKind::Packaged, folder, &denied()),
        None
    );
}

#[test]
fn the_explanation_exists_in_every_language() {
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = crate::gui::common::harness::locale(code);
        let text = state_folder_message(InstallKind::Portable, Path::new("some-folder"), &denied())
            .unwrap();
        assert!(text.contains("some-folder"), "{code}: {text}");
        assert!(
            !text.contains("state_folder_not_writable"),
            "{code}: untranslated"
        );
        let hint = gui_failure_message("Failed to create OpenGL context");
        assert!(hint.contains("OpenGL 2.1"), "{code}: {hint}");
        assert!(!hint.contains("graphics_hint"), "{code}: untranslated");
    }
}

#[test]
fn a_toolkit_failure_that_names_opengl_gets_the_driver_hint_and_another_does_not() {
    let _locale = crate::gui::common::harness::locale("en");
    for error in [
        "Failed to create OpenGL context: no pixel format",
        "wgl function was not loaded",
        "egl function was not loaded",
        "OPENGL32.dll could not be initialised",
    ] {
        let text = gui_failure_message(error);
        assert!(text.contains(error), "{text}");
        assert!(text.contains("OpenGL 2.1"), "{text}");
        assert!(text.contains("Microsoft Basic Display Adapter"), "{text}");
    }
    let plain = gui_failure_message("the window could not be created");
    assert!(plain.contains("the window could not be created"), "{plain}");
    assert!(!plain.contains("OpenGL 2.1"), "{plain}");
}
