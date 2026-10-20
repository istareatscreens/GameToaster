use super::super::harness;
use super::super::Dialog;
use super::*;
use crate::launcher_gui::dialogs::PluginChoice;
use crate::launcher_gui::model::GameId;

fn draft(close_mode: CloseMode, close_script: Option<PathBuf>) -> GameSettingsDraft {
    GameSettingsDraft {
        id: GameId(7),
        label: "Game".into(),
        watch_exe: String::new(),
        close_mode,
        close_script,
        config_targets: Vec::new(),
        store_picker: None,
        store_filter: String::new(),
        plugin: None,
        plugin_choice: PluginChoice::new(Vec::new()),
        plugin_inspection: None,
        confirm_config_reset: false,
        resolved_watch_name: None,
        automatic_watch_name: None,
        stored_automatic_watch: None,
    }
}

#[test]
fn game_settings_offers_cancel_then_save_and_save_needs_a_close_script() {
    let mut shown = harness::show(Dialog::GameSettings(draft(CloseMode::Script, None)));
    assert_eq!(shown.actions().len(), 2);
    assert!(
        matches!(shown.click_action(1), DialogEvent::None),
        "Save is disabled for the Script close mode without a script"
    );
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));

    let script = PathBuf::from("C:/games/close.bat");
    let mut shown = harness::show(Dialog::GameSettings(draft(
        CloseMode::Script,
        Some(script.clone()),
    )));
    match shown.click_action(1) {
        DialogEvent::SaveGameSettings {
            id,
            close_mode,
            close_script,
            ..
        } => {
            assert_eq!(id, GameId(7));
            assert_eq!(close_mode, CloseMode::Script);
            assert_eq!(close_script, Some(script));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_inspection_shows_each_file_s_first_original_and_the_transaction_count() {
    use crate::plugin::deployment::{
        AssignmentState, BackupRecord, FirstInstallOriginal, TransactionState,
    };
    use crate::plugin::workflow::{AssignmentInspection, ManagedFileInspection};

    let _locale = crate::gui::common::harness::locale("en");
    let file =
        |destination: &str, original: FirstInstallOriginal, backups: usize| ManagedFileInspection {
            kind: ManagedFileKind::Config,
            destination: PathBuf::from(destination),
            first_install_original: original,
            backup_chain: (0..backups)
                .map(|index| BackupRecord {
                    path: PathBuf::from(format!("C:/backups/{index}")),
                    sha256: "a".repeat(64),
                    reason: "update".into(),
                })
                .collect(),
        };
    let state = PluginInspectionState::Loaded(AssignmentInspection {
        assignment_id: "assignment-7".into(),
        state: AssignmentState::Active,
        latest_transaction_state: Some(TransactionState::Committed),
        transaction_count: 5,
        receipt_path: PathBuf::from("C:/ledger/receipt.toml"),
        receipt_only: false,
        active_files: vec![
            file(
                "C:/game/a.ini",
                FirstInstallOriginal::Existing {
                    backup: PathBuf::from("C:/backups/first-a.ini"),
                    sha256: "b".repeat(64),
                },
                2,
            ),
            file("C:/game/b.dll", FirstInstallOriginal::Absent, 0),
            file(
                "C:/game/c.ini",
                FirstInstallOriginal::Preserved {
                    sha256: "c".repeat(64),
                },
                1,
            ),
        ],
        retired_files: Vec::new(),
    });
    let context = egui::Context::default();
    crate::gui::common::theme::apply(&context);
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        show_assignment_inspection(ui, Some(&state));
    });
    let texts: Vec<String> = crate::gui::common::harness::painted_texts(&output)
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    assert!(
        texts
            .iter()
            .any(|text| text == "State: Active · Latest transaction: Committed · 5 transaction(s)"),
        "{texts:#?}"
    );

    let files = [
        "C:/game/a.ini",
        "First original: C:/backups/first-a.ini",
        "2 dated backup(s)",
        "C:/game/b.dll",
        "First original: file was absent",
        "0 dated backup(s)",
        "C:/game/c.ini",
        "Your original file was kept in place, so no backup was needed",
        "1 dated backup(s)",
    ];
    assert!(
        texts.windows(files.len()).any(|window| window == files),
        "{texts:#?}"
    );
}
