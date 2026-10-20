use std::path::PathBuf;

use eframe::egui;

use super::{Dialog, DialogEvent};
use crate::gui::common::harness::{
    click, footer_actions, painted_scroll_bars, painted_texts, settled, MAIN_WINDOW,
};
use crate::gui::common::theme;
use crate::launcher_gui::model::{CloseMode, GameEntry, GameId};
use crate::plugin::catalog::library::PluginDescriptor;
use crate::plugin::catalog::CatalogRevision;
use crate::plugin::manifest::Manifest;
use crate::plugin::workflow::PluginActionPreview;

const MODAL: &str = "launcher-themed-modal";

pub(super) fn main_window() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, MAIN_WINDOW)
}

pub(super) struct Shown {
    context: egui::Context,
    pub(super) dialog: Dialog,

    pub(super) output: egui::FullOutput,
    time: f64,
}

pub(super) fn show(mut dialog: Dialog) -> Shown {
    let context = egui::Context::default();
    theme::apply(&context);
    let output = settled(&context, Some(main_window()), |ui| {
        let _ = dialog.show(ui.ctx());
    });
    Shown {
        context,
        dialog,
        output,
        time: 1.1,
    }
}

impl Shown {
    pub(super) fn window(&self) -> egui::Rect {
        self.context
            .memory(|memory| memory.area_rect(egui::Id::new(MODAL)))
            .expect("the dialog is shown")
    }

    pub(super) fn bars(&self) -> Vec<(egui::Rect, egui::Color32)> {
        painted_scroll_bars(&self.output, &self.context.global_style().spacing.scroll)
    }

    pub(super) fn actions(&self) -> Vec<egui::Rect> {
        footer_actions(&self.output, self.window())
    }

    fn frame(&mut self, mut input: egui::RawInput) -> DialogEvent {
        self.time += 0.1;
        input.screen_rect = Some(main_window());
        input.time = Some(self.time);
        let mut event = DialogEvent::None;
        let dialog = &mut self.dialog;
        self.output = self
            .context
            .run_ui(input, |ui| event = dialog.show(ui.ctx()));
        event
    }

    pub(super) fn click_action(&mut self, index: usize) -> DialogEvent {
        let actions = self.actions();
        let target = actions
            .get(index)
            .unwrap_or_else(|| panic!("no action {index} among {actions:?}"))
            .center();
        self.frame(click(target, None, 0.0))
    }

    pub(super) fn click_text(&mut self, text: &str) -> DialogEvent {
        let target = painted_texts(&self.output)
            .into_iter()
            .find(|(painted, _)| painted == text)
            .map(|(_, rect)| rect.center())
            .unwrap_or_else(|| panic!("{text:?} is not painted"));
        self.frame(click(target, None, 0.0))
    }

    pub(super) fn hover_texts(&mut self, text: &str) -> Vec<String> {
        let target = painted_texts(&self.output)
            .into_iter()
            .find(|(painted, _)| painted == text)
            .map(|(_, rect)| rect.center())
            .unwrap_or_else(|| panic!("{text:?} is not painted"));

        for _ in 0..3 {
            self.idle();
        }
        self.frame(egui::RawInput {
            events: vec![egui::Event::PointerMoved(target)],
            ..Default::default()
        });
        for _ in 0..10 {
            self.idle();
        }
        painted_texts(&self.output)
            .into_iter()
            .map(|(painted, _)| painted)
            .collect()
    }

    pub(super) fn press(&mut self, key: egui::Key) -> DialogEvent {
        self.frame(egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        })
    }

    pub(super) fn type_text(&mut self, text: &str) -> DialogEvent {
        self.frame(egui::RawInput {
            events: vec![egui::Event::Text(text.to_owned())],
            ..Default::default()
        })
    }

    pub(super) fn focused(&self) -> Option<egui::Id> {
        self.context.memory(|memory| memory.focused())
    }

    pub(super) fn idle(&mut self) -> DialogEvent {
        self.frame(egui::RawInput::default())
    }
}

pub(super) fn plugin_action_preview() -> PluginActionPreview {
    use crate::plugin::deployment::{ModificationPurpose, UserModification};
    use crate::plugin::paths::{AmbiguityKind, ExternalGrant, ResolutionAmbiguity};
    PluginActionPreview {
        token: "0123456789abcdef0123456789abcdef".into(),
        action: crate::plugin::workflow::WorkflowAction::Install,
        plugin_id: "org.example.plugin".into(),
        plugin_version: semver::Version::parse("1.2.3").unwrap(),
        plugin_md5: "a".repeat(32),
        plugin_sha256: "b".repeat(64),
        label: "Example".into(),
        description: "Fixture".into(),
        selected_target: r"C:\Games\game.exe".into(),
        resolved_launcher: None,
        resolved_watch_target: None,
        files: Vec::new(),
        install_scripts: Vec::new(),
        required_external_grants: vec![ExternalGrant {
            plugin_id: "org.example.plugin".into(),
            game_id: 7,
            revision_sha256: "b".repeat(64),
            destination_root: r"C:\Shared".into(),
            root_identity: None,
        }],
        ambiguities: vec![ResolutionAmbiguity {
            kind: AmbiguityKind::Launcher,
            candidates: vec![r"C:\Games\runner.exe".into()],
        }],
        modifications: vec![UserModification {
            destination: r"C:\Games\config.ini".into(),
            purpose: ModificationPurpose::ReplaceWithPlugin,
            last_deployed_sha256: "c".repeat(64),
            current_sha256: Some("d".repeat(64)),
        }],
        assignment_state: None,
        latest_transaction_state: None,
        transaction_count: 0,
    }
}

pub(super) fn game_entry() -> GameEntry {
    GameEntry {
        id: GameId(7),
        label: "Original".into(),
        controller_image: None,
        selected_target: r"C:\games\game.exe".into(),
        plugin: None,
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: vec![PathBuf::from(r"C:\games\game.ini")],
    }
}

pub(super) fn plugin_descriptor() -> PluginDescriptor {
    let manifest = Manifest::parse(
        r#"manifestVersion = 1
pluginId = "org.example.patcher"
pluginVersion = "1.2.0"
label = "Patcher fixture"
description = "Patches the selected game before each launch and restores it afterwards."
"#,
    )
    .unwrap();
    PluginDescriptor {
        revision: CatalogRevision {
            plugin_id: manifest.plugin_id.clone(),
            plugin_version: manifest.plugin_version.clone(),
            package_md5: "e".repeat(32),
            package_sha256: "f".repeat(64),
            imported_at: 0,
            root: PathBuf::from("C:/catalog/patcher"),
        },
        manifest,
    }
}
