use super::harness::{game_entry, main_window, plugin_action_preview, plugin_descriptor};
use super::*;
use crate::gui::common::components::DialogContent;
use crate::gui::common::harness::{scaffold_width, settled_modal, SettledModal};
use crate::gui::common::theme;
use crate::plugin::catalog::library::PluginDescriptor;

const EGUI_DEFAULT_BODY_HEIGHT: f32 = 420.0;

fn add_draft(plugin: Option<PluginDescriptor>) -> AddDraft {
    let selected = plugin.as_ref().map(|_| 0);
    let mut draft = AddDraft::new(
        PathBuf::from("C:/games/arcade/game.exe"),
        "game".into(),
        plugin.into_iter().collect(),
    );
    draft.plugin_choice.selected = selected;
    draft
}

fn add_content_height(draft: &AddDraft) -> f32 {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut draft = draft.clone();
    let mut event = DialogEvent::None;
    let mut height = 0.0;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        theme::apply_widget_visuals(ui);
        height = ui
            .scope(|ui| {
                ui.set_width(490.0);
                let mut dialog = add::AddDialog::new(&mut draft, &mut event);
                dialog.body(ui);
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| dialog.actions(ui));
            })
            .response
            .rect
            .height();
    });
    height
}

fn shown(mut dialog: Dialog) -> SettledModal {
    settled_modal(egui::Id::new("launcher-themed-modal"), |context| {
        let _ = dialog.show(context);
    })
}

#[test]
fn the_add_game_dialog_shows_its_whole_body_without_a_scroll_bar() {
    for plugin in [None, Some(plugin_descriptor())] {
        let draft = add_draft(plugin);
        let content = add_content_height(&draft);
        assert!(
            content > EGUI_DEFAULT_BODY_HEIGHT,
            "the body and footer ({content}) must be taller than egui's default window body to cover the defect"
        );
        let modal = shown(Dialog::Add(draft));
        assert!(modal.bars.is_empty(), "{:?}", modal.bars);
        assert!(
            main_window().contains_rect(modal.window),
            "{:?}",
            modal.window
        );
    }
}

fn inspected_game_settings() -> GameSettingsDraft {
    use crate::plugin::deployment::{AssignmentState, FirstInstallOriginal, TransactionState};
    use crate::plugin::workflow::{AssignmentInspection, ManagedFileInspection};
    let long = PathBuf::from(
        "C:/Games/Arcade Collection/A Very Long Folder Name For The Test/Subfolder/config_file.ini",
    );
    let file = |original| ManagedFileInspection {
        kind: crate::plugin::paths::ManagedFileKind::Config,
        destination: long.clone(),
        first_install_original: original,
        backup_chain: Vec::new(),
    };
    GameSettingsDraft {
        id: crate::launcher_gui::model::GameId(7),
        label: "Game".into(),
        watch_exe: String::new(),
        close_mode: crate::launcher_gui::model::CloseMode::KillExe,
        close_script: None,
        config_targets: vec![long.clone(), PathBuf::from("C:/Games/b.ini")],
        store_picker: None,
        store_filter: String::new(),
        plugin: None,
        plugin_choice: PluginChoice {
            selected: Some(0),
            ..PluginChoice::new(vec![plugin_descriptor()])
        },
        plugin_inspection: Some(PluginInspectionState::Loaded(AssignmentInspection {
            assignment_id: "assignment-0123456789abcdef".into(),
            state: AssignmentState::Active,
            latest_transaction_state: Some(TransactionState::Committed),
            transaction_count: 12,
            receipt_path: long.clone(),
            receipt_only: false,
            active_files: vec![
                file(FirstInstallOriginal::Existing {
                    backup: long.clone(),
                    sha256: "b".repeat(64),
                }),
                file(FirstInstallOriginal::Absent),
                file(FirstInstallOriginal::Preserved {
                    sha256: "c".repeat(64),
                }),
            ],
            retired_files: Vec::new(),
        })),
        confirm_config_reset: false,
        resolved_watch_name: None,
        automatic_watch_name: None,
        stored_automatic_watch: None,
    }
}

fn scripted_game_settings() -> GameSettingsDraft {
    let mut draft = inspected_game_settings();
    draft.close_mode = crate::launcher_gui::model::CloseMode::Script;
    draft.close_script = Some(PathBuf::from(
        "C:/Games/Arcade Collection/A Very Long Folder Name For The Test/Scripts/close_the_game.bat",
    ));
    draft
}

fn manifest_with_an_argument() -> ManifestEditorDraft {
    let mut draft = ManifestEditorDraft::new(plugin_descriptor()).unwrap();
    draft
        .manifest
        .install_scripts
        .push(crate::plugin::manifest::InstallScript {
            install_script_location: "scripts/setup.bat".into(),
            install_script_arguments: vec!["--quiet".into()],
        });
    draft
}

fn every_dialog() -> Vec<(&'static str, Dialog)> {
    let mut config_game = game_entry();
    config_game.config_targets = vec![
        PathBuf::from("C:/games/a.ini"),
        PathBuf::from("C:/games/b.ini"),
    ];
    let hotkeys = HotkeyDraft {
        games: (0..20)
            .map(|index| HotkeyRow {
                id: crate::launcher_gui::model::GameId(index),
                label: format!("Game {index}"),
                launch: None,
                close: None,
            })
            .collect(),
        ..HotkeyDraft::default()
    };
    let long_error = (0..80)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    vec![
        ("add", Dialog::Add(add_draft(None))),
        (
            "add with a plugin",
            Dialog::Add(add_draft(Some(plugin_descriptor()))),
        ),
        ("remove", Dialog::Remove(game_entry())),
        (
            "edit label",
            Dialog::Presentation(PresentationDraft::new(game_entry(), None, None)),
        ),
        ("hotkeys", Dialog::Hotkeys(hotkeys)),
        (
            "game settings",
            Dialog::GameSettings(inspected_game_settings()),
        ),
        (
            "game settings closing with a script",
            Dialog::GameSettings(scripted_game_settings()),
        ),
        ("open config", Dialog::ConfigMenu(config_game)),
        (
            "plugins",
            Dialog::PluginLibrary(PluginLibraryDraft::new(
                vec![plugin_descriptor()],
                None,
                &[],
            )),
        ),
        (
            "manifest editor",
            Dialog::ManifestEditor(Box::new(
                ManifestEditorDraft::new(plugin_descriptor()).unwrap(),
            )),
        ),
        (
            "manifest editor with a script argument",
            Dialog::ManifestEditor(Box::new(manifest_with_an_argument())),
        ),
        (
            "review plugin changes",
            Dialog::PluginAction(PluginActionDraft::new(plugin_action_preview())),
        ),
        ("error", Dialog::error("Title", "One line.")),
        ("long error", Dialog::error("Title", long_error)),
        ("notice", Dialog::notice("Title", "One line.")),
        (
            "configuration notice",
            config_notice(&crate::launcher_gui::config::Notice::Upgraded {
                kept: PathBuf::from(
                    r"D:\Arcade\GameToaster\State\Packages\GameToaster_0123456789abc\LocalState\GameToaster.ini.schema-2.20261018T093000.123Z.bak",
                ),
                plugin_needed: vec!["First Game".into(), "Second Game".into()],
            }),
        ),
    ]
}

fn width_with_the_launcher_fonts(dialog: &mut Dialog) -> f32 {
    let context = egui::Context::default();
    theme::apply(&context);
    theme::install_fonts(&context);
    let _ = crate::gui::common::harness::settled(&context, Some(main_window()), |ui| {
        let _ = dialog.show(ui.ctx());
    });
    context
        .memory(|memory| memory.area_rect(egui::Id::new("launcher-themed-modal")))
        .expect("the dialog is shown")
        .width()
}

#[test]
fn every_dialog_keeps_the_scaffolds_width_in_every_language() {
    let expected = scaffold_width();
    let mut wider = Vec::new();
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = crate::gui::common::harness::locale(code);
        for (name, mut dialog) in every_dialog() {
            let width = width_with_the_launcher_fonts(&mut dialog);
            if (width - expected).abs() >= 0.5 {
                wider.push(format!("{code}, {name}: {width}"));
            }
        }
    }
    assert!(
        wider.is_empty(),
        "the scaffold's width is {expected}: {wider:#?}"
    );
}

#[test]
fn a_dialog_taller_than_its_limit_scrolls_624_points_with_a_visible_handle() {
    let message = (0..80)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let shown = super::harness::show(Dialog::error("Long error", message));
    let bars = shown.bars();
    assert_eq!(bars.len(), 2, "{bars:?}");
    let ((track_rect, track), (_, handle)) = (bars[0], bars[1]);
    assert!((track_rect.height() - 624.0).abs() < 0.01, "{track_rect:?}");
    assert_eq!((track, handle), (theme::ICON_TILE, theme::SCROLL_HANDLE));
    let actions = shown.actions();
    assert_eq!(actions.len(), 2, "{actions:?}");
    for action in actions {
        assert!(
            action.top() > track_rect.bottom(),
            "{action:?} {track_rect:?}"
        );
        assert!(shown.window().contains_rect(action), "{action:?}");
    }
    assert!(
        main_window().contains_rect(shown.window()),
        "{:?}",
        shown.window()
    );
}
