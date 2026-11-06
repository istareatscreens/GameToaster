use super::super::harness;
use super::super::Dialog;
use super::*;
use crate::launcher_gui::dialogs::PluginChoice;
use crate::launcher_gui::model::GameId;

fn draft(pre_start_script: Option<PathBuf>, close_script: Option<PathBuf>) -> GameSettingsDraft {
    GameSettingsDraft {
        id: GameId(7),
        label: "Game".into(),
        watch_exe: String::new(),
        pre_start_script,
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

fn saved_scripts(shown: &mut harness::Shown) -> (Option<PathBuf>, Option<PathBuf>) {
    match shown.click_action(1) {
        DialogEvent::SaveGameSettings {
            id,
            pre_start_script,
            close_script,
            ..
        } => {
            assert_eq!(id, GameId(7));
            (pre_start_script, close_script)
        }
        other => panic!("{other:?}"),
    }
}

fn texts(shown: &harness::Shown) -> Vec<String> {
    crate::gui::common::harness::painted_texts(&shown.output)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

#[test]
fn game_settings_offers_cancel_then_save_and_save_never_waits_for_a_script() {
    let mut shown = harness::show(Dialog::GameSettings(draft(None, None)));
    assert_eq!(shown.actions().len(), 2);
    assert_eq!(saved_scripts(&mut shown), (None, None));
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
}

#[test]
fn saving_carries_both_scripts() {
    let start = PathBuf::from("C:/games/start.bat");
    let close = PathBuf::from("C:/games/close.bat");
    let mut shown = harness::show(Dialog::GameSettings(draft(
        Some(start.clone()),
        Some(close.clone()),
    )));
    assert_eq!(saved_scripts(&mut shown), (Some(start), Some(close)));
}

#[test]
fn both_script_settings_are_shown_and_the_close_action_choice_is_gone() {
    let _locale = crate::gui::common::harness::locale("en");
    let shown = harness::show(Dialog::GameSettings(draft(None, None)));
    let texts = texts(&shown);
    for expected in [
        "Pre Start Script",
        "Runs before the game starts. The game launches once it has finished; a script that fails stops the launch.",
        "Close Script",
        "Runs after the game has closed, and after a launch that stopped once the pre-start script had run.",
    ] {
        assert!(texts.iter().any(|text| text == expected), "{expected:?} in {texts:#?}");
    }
    let count = |wanted: &str| texts.iter().filter(|text| *text == wanted).count();
    assert_eq!(count("No script selected"), 2, "{texts:#?}");
    assert_eq!(count("Choose .bat"), 2, "{texts:#?}");

    for gone in ["Clear", "Close action", "Kill game", "Run script"] {
        assert_eq!(count(gone), 0, "{gone:?} in {texts:#?}");
    }
}

#[test]
fn clear_removes_only_its_own_script() {
    let _locale = crate::gui::common::harness::locale("en");
    let start = PathBuf::from("C:/games/start.bat");
    let close = PathBuf::from("C:/games/close.bat");

    let mut shown = harness::show(Dialog::GameSettings(draft(Some(start.clone()), None)));
    shown.click_text("Clear");
    assert_eq!(saved_scripts(&mut shown), (None, None));

    let mut shown = harness::show(Dialog::GameSettings(draft(None, Some(close.clone()))));
    shown.click_text("Clear");
    assert_eq!(saved_scripts(&mut shown), (None, None));

    let mut shown = harness::show(Dialog::GameSettings(draft(
        Some(start.clone()),
        Some(close.clone()),
    )));
    assert_eq!(
        texts(&shown).iter().filter(|text| *text == "Clear").count(),
        2
    );
    shown.click_text("Clear");
    assert_eq!(saved_scripts(&mut shown), (None, Some(close)));
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
