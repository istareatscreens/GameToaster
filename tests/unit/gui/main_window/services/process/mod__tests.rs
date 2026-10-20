use super::*;
use crate::support::process::console_policy::{ConsolePlan, ConsoleWindow};
use crate::support::process::fixtures::{ends_within, hold};
use crate::support::process::interpreter::comspec;
use std::path::{Path, PathBuf};
use std::time::Duration;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[test]
fn job_shutdown_terminates_root_child_and_grandchild() {
    let powershell = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    if !powershell.is_file() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("gametoaster-job-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let child_file = dir.join("child.pid");
    let grandchild_file = dir.join("grandchild.pid");
    let child_script = dir.join("child.ps1");
    std::fs::write(
        &child_script,
        format!(
            "$grandchild=Start-Process ping.exe -ArgumentList '127.0.0.1','-n','60' -PassThru\n\
             Set-Content -LiteralPath '{}' -Value $grandchild.Id\n\
             Wait-Process -Id $grandchild.Id\n",
            grandchild_file.display()
        ),
    )
    .unwrap();
    let script = format!(
        "$child=Start-Process -FilePath '{}' -ArgumentList '-NoProfile','-File','{}' -PassThru; \
         Set-Content -LiteralPath '{}' -Value $child.Id; Wait-Process -Id $child.Id",
        powershell.display(),
        child_script.display(),
        child_file.display(),
    );
    let mut manager = ProcessManager::with_completion_sink(Arc::new(|_| {}));
    let spawned = manager
        .spawn(
            GameId(1),
            &SpawnSpec {
                executable: powershell,
                working_directory: dir.clone(),
                arguments: vec!["-NoProfile".into(), "-Command".into(), script.into()],
                console: ConsolePlan {
                    window: ConsoleWindow::Hidden,
                    pause_on_failure: false,
                },
                environment: Vec::new(),
            },
        )
        .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let (child, grandchild) = loop {
        let child = read_pid(&child_file);
        let grandchild = read_pid(&grandchild_file);
        if child > 0 && grandchild > 0 {
            break (child, grandchild);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "process tree PIDs were not recorded before timeout: child={child}, grandchild={grandchild}, root_running={}",
            pid_is_running(spawned.pid)
        );
        std::thread::sleep(Duration::from_millis(50));
    };

    let held = [
        ("root", hold(spawned.pid)),
        ("child", hold(child)),
        ("grandchild", hold(grandchild)),
    ];
    manager.shutdown().unwrap();

    for (name, process) in &held {
        assert!(
            ends_within(process, Duration::from_secs(10)),
            "{name} PID {} survived Job Object shutdown",
            process.pid()
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn natural_exit_is_reported_by_job_completion_port() {
    let command = comspec();
    if !command.is_file() {
        return;
    }
    let dir = std::env::temp_dir();
    let (completion_tx, completion_rx) = std::sync::mpsc::sync_channel(1);
    let mut manager = ProcessManager::with_completion_sink(Arc::new(move |event| {
        let _ = completion_tx.try_send(event);
    }));
    manager
        .spawn(
            GameId(99),
            &SpawnSpec {
                executable: command,
                working_directory: dir,
                arguments: vec!["/C".into(), "exit 0".into()],
                console: ConsolePlan {
                    window: ConsoleWindow::Hidden,
                    pause_on_failure: false,
                },
                environment: Vec::new(),
            },
        )
        .unwrap();
    assert_eq!(
        completion_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        ProcessCompletion::Exited(GameId(99))
    );
    assert!(completion_rx
        .recv_timeout(Duration::from_millis(100))
        .is_err());
    assert!(manager.release_completed(GameId(99)));
    assert!(!manager.release_completed(GameId(99)));
}

#[test]
fn a_spawned_process_reports_when_it_was_created() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = ProcessManager::with_completion_sink(Arc::new(|_| {}));
    let spawned = manager
        .spawn(
            GameId(7),
            &SpawnSpec {
                executable: comspec(),
                working_directory: dir.path().to_path_buf(),
                arguments: vec!["/d".into(), "/c".into(), "ping -n 20 127.0.0.1 >nul".into()],
                console: ConsolePlan {
                    window: ConsoleWindow::Windowless,
                    pause_on_failure: false,
                },
                environment: Vec::new(),
            },
        )
        .unwrap();
    let opened =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, spawned.pid) }.unwrap();
    assert_eq!(
        crate::support::process::handle::creation_time(opened),
        Some(spawned.created)
    );
    unsafe {
        let _ = CloseHandle(opened);
    }
    manager.shutdown().unwrap();
}

fn read_pid(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
}

fn pid_is_running(pid: u32) -> bool {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return false;
    };
    let mut exit_code = 0;
    let result = unsafe { GetExitCodeProcess(handle, &mut exit_code) }.is_ok();
    unsafe {
        let _ = CloseHandle(handle);
    }
    result && exit_code == 259
}
