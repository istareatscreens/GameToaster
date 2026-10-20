use super::*;
use crate::plugin::deployment::AssignmentState;
use crate::plugin::manifest::{InstallScript, Manifest};
use semver::Version;

struct Setup {
    _root: tempfile::TempDir,
    package: PathBuf,
    game: PathBuf,
    store: LedgerStore,
    manifest: Manifest,
    ledger: AssignmentLedger,
}

fn setup(name: &str, text: &str, arguments: &[&str]) -> Setup {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("package");
    let game = root.path().join("game");
    let state = root.path().join("state");
    fs::create_dir_all(&package).unwrap();
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("game.exe"), b"game").unwrap();
    fs::write(package.join(name), text).unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        plugin_id: "org.example.script".into(),
        plugin_version: Version::new(1, 0, 0),
        label: "Script".into(),
        description: "Install script test".into(),
        is_patcher: false,
        game_launcher: None,
        executable_watch: None,
        executable_watch_type: None,
        runtime: None,
        config_files: Vec::new(),
        plugin_files: Vec::new(),
        install_scripts: vec![InstallScript {
            install_script_location: name.into(),
            install_script_arguments: arguments
                .iter()
                .map(|argument| argument.to_string())
                .collect(),
        }],
    };
    let ledger = AssignmentLedger {
        ledger_version: 1,
        assignment_id: "a".repeat(32),
        plugin_id: manifest.plugin_id.clone(),
        plugin_version: manifest.plugin_version.to_string(),
        revision_sha256: "b".repeat(64),
        game_id: 7,
        game_folder: fs::canonicalize(&game).unwrap(),
        selected_target: fs::canonicalize(game.join("game.exe")).unwrap(),
        resolved_launcher: None,
        resolved_watch_target: None,
        state: AssignmentState::Active,
        files: Vec::new(),
        retired_files: Vec::new(),
        transactions: Vec::new(),
        install_script_receipts: Vec::new(),
    };
    let store = LedgerStore::new(state).unwrap();
    Setup {
        _root: root,
        package,
        game,
        store,
        manifest,
        ledger,
    }
}

#[test]
fn successful_script_has_a_durable_once_only_receipt() {
    let Setup {
        _root,
        package,
        game,
        store,
        manifest,
        mut ledger,
    } = setup(
        "install.cmd",
        "@echo off\r\nset /a n=0\r\nif exist count.txt set /p n=<count.txt\r\nset /a n+=1\r\n>count.txt echo %n%\r\n",
        &[],
    );

    run_pending(&store, &package, &manifest, &mut ledger).unwrap();
    run_pending(&store, &package, &manifest, &mut ledger).unwrap();

    assert_eq!(
        fs::read_to_string(game.join("count.txt")).unwrap().trim(),
        "1"
    );
    assert_eq!(ledger.install_script_receipts.len(), 1);
    assert_eq!(
        ledger.install_script_receipts[0].state,
        InstallScriptRunState::Succeeded
    );
    assert_eq!(store.load(7, &"a".repeat(32)).unwrap(), ledger);

    ledger.install_script_receipts[0].state = InstallScriptRunState::Started;
    store.save(&ledger).unwrap();
    let error = run_pending(&store, &package, &manifest, &mut ledger).unwrap_err();
    assert!(error.contains("will not be started again"), "{error}");
    assert_eq!(
        fs::read_to_string(game.join("count.txt")).unwrap().trim(),
        "1"
    );
}

#[test]
fn a_script_receives_its_arguments_exactly_as_written() {
    let Setup {
        _root,
        package,
        game,
        store,
        manifest,
        mut ledger,
    } = setup(
        "install.cmd",
        "@echo off\r\n>args.txt (echo %1& echo %2& echo %3)\r\n",
        &["a&b", "%PATH%", "c^d"],
    );

    run_pending(&store, &package, &manifest, &mut ledger).unwrap();

    assert_eq!(
        fs::read_to_string(game.join("args.txt")).unwrap(),
        "\"a&b\"\r\n\"%PATH%\"\r\n\"c^d\"\r\n"
    );
}
