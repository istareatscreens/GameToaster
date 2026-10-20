use super::super::harness::{self, plugin_descriptor};
use super::super::Dialog;
use super::*;
use crate::gui::common::harness::{assert_pinned, locale};

#[test]
fn an_invalid_manifest_imports_nothing_and_pins_its_error() {
    let _locale = locale("en");
    let mut draft = ManifestEditorDraft::new(plugin_descriptor()).unwrap();
    draft.version_text = "not a version".into();
    let mut shown = harness::show(Dialog::ManifestEditor(Box::new(draft)));
    assert_eq!(shown.actions().len(), 2);
    assert!(
        matches!(shown.click_action(1), DialogEvent::None),
        "Validate & import refuses an invalid manifest"
    );
    for _ in 0..3 {
        let _ = shown.idle();
    }
    let Dialog::ManifestEditor(draft) = &shown.dialog else {
        unreachable!("the dialog is still the manifest editor")
    };
    let error = draft
        .error
        .clone()
        .expect("Validate & import records the error");
    assert_pinned(
        &shown.output,
        shown.window(),
        &t!("title.plugin_manifest_editor"),
        &error,
    );
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
}
