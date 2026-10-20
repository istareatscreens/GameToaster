use super::*;
use crate::launcher_gui::model::{CloseMode, GameId};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

const HIDDEN: ConsoleSettings = ConsoleSettings {
    show_command_windows: false,
    pause_on_failure: false,
};
const SHOWN: ConsoleSettings = ConsoleSettings {
    show_command_windows: true,
    pause_on_failure: false,
};
const PAUSE: ConsoleSettings = ConsoleSettings {
    show_command_windows: false,
    pause_on_failure: true,
};

fn entry(path: PathBuf) -> GameEntry {
    GameEntry {
        id: GameId(1),
        label: "Script".into(),
        controller_image: None,
        selected_target: path,
        plugin: None,
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    }
}

#[test]
fn launches_bat_script_through_cmd_even_with_spaces() {
    let base = std::env::temp_dir().join(format!("gametoaster-bat-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let script = base.join("run me.bat");
    let marker = base.join("marker.txt");
    fs::write(
        &script,
        format!("@echo off\r\necho ok> \"{}\"\r\n", marker.display()),
    )
    .unwrap();
    let runtime = RuntimeService::new(base.clone());
    let mut processes = ProcessManager::with_completion_sink(Arc::new(|_| {}));
    runtime
        .launch(&entry(script), HIDDEN, &mut processes)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    while !marker.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        marker.is_file(),
        "the .bat script never produced its marker"
    );
    let _ = processes.shutdown();
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn generic_plan_has_no_plugin_asset_dependency() {
    let runtime = RuntimeService::new(std::env::temp_dir());
    let missing = entry(PathBuf::from(r"C:\missing-for-plan.exe"));
    assert!(runtime.prepare(&missing, HIDDEN).is_err());
}

#[test]
fn the_launch_plan_takes_the_console_policy_for_the_target_kind() {
    let base = tempfile::tempdir().unwrap();
    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    let console = base.path().join("console game.exe");
    let window = base.path().join("window game.exe");
    let script = base.path().join("start game.bat");
    fs::copy(system.join("cmd.exe"), &console).unwrap();
    fs::copy(system.join("notepad.exe"), &window).unwrap();
    fs::write(&script, "@rem start\r\n").unwrap();
    let runtime = RuntimeService::new(base.path().to_path_buf());
    for (target, kind) in [
        (&console, ProgramKind::Console),
        (&window, ProgramKind::Graphical),
        (&script, ProgramKind::Script),
    ] {
        for settings in [HIDDEN, SHOWN, PAUSE] {
            let plan = runtime.prepare(&entry(target.clone()), settings).unwrap();
            assert_eq!(
                plan.console,
                console_policy::plan(kind, settings),
                "{kind:?} {settings:?}"
            );
        }
    }
}

#[test]
fn a_script_in_a_folder_cmd_reads_specially_runs_as_written() {
    let base = tempfile::tempdir().unwrap();
    let folder = base.path().join("a&b %USERNAME% (x)");
    fs::create_dir_all(&folder).unwrap();
    let script = folder.join("run.bat");
    let marker = folder.join("marker.txt");
    fs::write(&script, "@echo ok> \"%~dp0marker.txt\"\r\n").unwrap();
    let runtime = RuntimeService::new(base.path().to_path_buf());
    let mut processes = ProcessManager::with_completion_sink(Arc::new(|_| {}));
    runtime
        .launch(&entry(script), HIDDEN, &mut processes)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    while !marker.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(marker.is_file(), "the script never ran from its folder");
    let _ = processes.shutdown();
}
