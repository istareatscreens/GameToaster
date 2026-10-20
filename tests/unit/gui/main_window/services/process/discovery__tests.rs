use super::*;

#[test]
fn process_name_matching_ignores_case_and_exe_suffix() {
    assert!(process_name_matches("game.exe", "game"));
    assert!(process_name_matches("GAME.EXE", "game.exe"));
    assert!(process_name_matches("game.exe", "game.exe"));
    assert!(!process_name_matches("other.exe", "game"));
    assert!(!process_name_matches("anything.exe", ""));
}

#[test]
fn exact_path_matching_rejects_same_named_executable_in_another_directory() {
    assert!(process_path_matches(
        Path::new(r"C:\Games\A\game.exe"),
        Path::new(r"c:/games/a/GAME.EXE")
    ));
    assert!(!process_path_matches(
        Path::new(r"C:\Games\B\game.exe"),
        Path::new(r"C:\Games\A\game.exe")
    ));
}

#[test]
fn a_running_program_is_found_by_name_and_by_path_and_ended_with_what_it_started() {
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let folder = tempfile::tempdir().unwrap();
    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    let name = format!("gt-discovery-{}.exe", std::process::id());
    let program = folder.path().join(&name);
    std::fs::copy(system.join("cmd.exe"), &program).unwrap();
    let mut root = Command::new(&program)
        .args(["/d", "/c", "ping -n 30 127.0.0.1 >nul"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .unwrap();
    let pid = root.id();
    let started_ping = || {
        crate::support::process::snapshot::snapshot()
            .into_iter()
            .filter(|entry| entry.parent == pid)
            .map(|entry| entry.pid)
            .filter(|child| pids_by_name("PING.EXE").contains(child))
            .collect::<Vec<u32>>()
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    let ping = loop {
        if let Some(&ping) = started_ping().first() {
            break ping;
        }
        assert!(Instant::now() < deadline, "the program never started ping");
        std::thread::sleep(Duration::from_millis(50));
    };

    assert!(is_process_running(&name));
    assert!(is_process_running(&name.to_ascii_uppercase()));
    assert_eq!(pids_by_name(&name), [pid]);
    assert_eq!(find_pid_by_name(&name), Some(pid));
    assert_eq!(pids_by_path(&program), [pid]);
    assert_eq!(find_pid_by_path(&program), Some(pid));
    assert!(is_process_path_running(&program));
    let elsewhere = folder.path().join("elsewhere").join(&name);
    assert!(!is_process_path_running(&elsewhere));
    assert_eq!(terminate_by_path(&elsewhere), 0);

    assert!(
        terminate_by_name(&name) >= 2,
        "the program and the ping it started"
    );
    let _ = root.wait();
    let deadline = Instant::now() + Duration::from_secs(10);
    while pids_by_name("PING.EXE").contains(&ping) {
        assert!(
            Instant::now() < deadline,
            "the ping it started was not ended"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!is_process_running(&name));
    assert_eq!(find_pid_by_name(&name), None);
}
