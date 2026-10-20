use super::super::{harness, Dialog};
use super::*;
use crate::plugin::catalog::library::PluginDescriptor;
use crate::plugin::catalog::CatalogRevision;
use crate::plugin::manifest::Manifest;
use std::path::PathBuf;

fn descriptor(executable_watch: Option<&str>) -> PluginDescriptor {
    let watch_line = executable_watch
        .map(|value| format!("executableWatch = \"{value}\"\n"))
        .unwrap_or_default();
    let manifest = Manifest::parse(&format!(
        "manifestVersion = 1\n\
         pluginId = \"org.example.add\"\n\
         pluginVersion = \"1.0.0\"\n\
         label = \"Add fixture\"\n\
         description = \"Add fixture\"\n\
         {watch_line}"
    ))
    .unwrap();
    PluginDescriptor {
        revision: CatalogRevision {
            plugin_id: manifest.plugin_id.clone(),
            plugin_version: manifest.plugin_version.clone(),
            package_md5: "e".repeat(32),
            package_sha256: "f".repeat(64),
            imported_at: 0,
            root: PathBuf::from("C:/catalog/add"),
        },
        manifest,
    }
}

fn drawn(draft: &mut AddDraft) -> Vec<String> {
    let context = egui::Context::default();
    let mut event = DialogEvent::None;
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        AddDialog::new(draft, &mut event).body(ui);
    });
    assert!(matches!(event, DialogEvent::None));
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_string()),
            _ => None,
        })
        .collect()
}

fn draft(path: &str, plugin: Option<PluginDescriptor>) -> AddDraft {
    let selected = plugin.as_ref().map(|_| 0);
    let mut draft = AddDraft::new(
        PathBuf::from(path),
        "Game".into(),
        plugin.into_iter().collect(),
    );
    draft.plugin_choice.selected = selected;
    draft
}

#[test]
fn the_dialog_follows_the_plugin_choice_into_the_watch_field() {
    let mut draft = draft(
        "C:/games/game.exe",
        Some(descriptor(Some("bin/patched_game.exe"))),
    );
    assert_eq!(draft.watch_exe, "game.exe", "filled in by the executable");
    let texts = drawn(&mut draft);
    assert_eq!(draft.watch_exe, "patched_game.exe", "{texts:?}");
    draft.plugin_choice.selected = None;
    let _ = drawn(&mut draft);
    assert_eq!(draft.watch_exe, "game.exe");
}

#[test]
fn add_dialog_shows_the_resolved_watch_name_as_the_placeholder_when_a_plugin_is_selected() {
    let mut with_plugin = draft(
        "C:/games/start.bat",
        Some(descriptor(Some("bin/patched_game.exe"))),
    );
    let texts = drawn(&mut with_plugin);
    assert!(
        texts.iter().any(|text| text == "patched_game.exe"),
        "{texts:?}"
    );
    assert!(
        with_plugin.label_focused,
        "the label field takes focus once"
    );

    let mut plain = draft("C:/games/start.bat", None);
    let texts = drawn(&mut plain);
    assert!(!texts.iter().any(|text| text == "start.bat"), "{texts:?}");
    assert!(plain.resolved_watch_name().is_none());
}

#[test]
fn a_second_frame_does_not_pull_focus_back_to_the_label_field() {
    let mut draft = draft("C:/games/game.exe", None);
    let context = egui::Context::default();
    let mut event = DialogEvent::None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        AddDialog::new(&mut draft, &mut event).body(ui)
    });
    assert!(draft.label_focused);

    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        AddDialog::new(&mut draft, &mut event).body(ui)
    });
    assert!(draft.label_focused);
}

#[test]
fn add_game_offers_cancel_then_continue_and_continue_needs_a_label() {
    let mut shown = harness::show(Dialog::Add(draft("C:/games/game.exe", None)));
    assert_eq!(shown.actions().len(), 2);
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
    match shown.click_action(1) {
        DialogEvent::AddContinue(added) => assert_eq!(added.label, "Game"),
        other => panic!("{other:?}"),
    }

    let mut blank = draft("C:/games/game.exe", None);
    blank.label = "   ".into();
    let mut shown = harness::show(Dialog::Add(blank));
    assert!(
        matches!(shown.click_action(1), DialogEvent::None),
        "Continue is disabled without a label"
    );
}

#[test]
fn enter_in_the_label_field_continues_with_the_trimmed_label() {
    let mut padded = draft("C:/games/game.exe", None);
    padded.label = "  Game  ".into();
    let mut shown = harness::show(Dialog::Add(padded));
    match shown.press(egui::Key::Enter) {
        DialogEvent::AddContinue(added) => assert_eq!(added.label, "Game"),
        other => panic!("{other:?}"),
    }
}
