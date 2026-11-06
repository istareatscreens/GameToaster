use std::sync::mpsc::sync_channel;
use std::time::Duration;

use super::*;
use crate::gui::main_window::app::launch_monitor::Presence;

#[test]
fn a_followed_program_that_ends_is_reported_gone() {
    let folder = tempfile::tempdir().unwrap();
    let system = std::env::var_os("SystemRoot").expect("SystemRoot is set");
    let program = folder
        .path()
        .join(format!("gametoaster-followed-{}.exe", std::process::id()));
    std::fs::copy(
        std::path::Path::new(&system)
            .join("System32")
            .join("PING.EXE"),
        &program,
    )
    .unwrap();
    let mut running = std::process::Command::new(&program)
        .args(["-n", "60", "127.0.0.1"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let (sender, receiver) = sync_channel(8);
    let watch = ProgramWatch::start_with(
        GameId(3),
        WatchedProgram {
            name: program.file_name().unwrap().to_string_lossy().into_owned(),
            path: Some(program.clone()),
        },
        sender,
        Duration::from_secs(1),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    assert!(
        receiver.try_recv().is_err(),
        "nothing to report while it runs"
    );

    running.kill().unwrap();
    running.wait().unwrap();
    let command = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(
        matches!(
            command,
            WorkerCommand::ProgramPresence {
                game_id: GameId(3),
                presence: Presence::Gone
            }
        ),
        "{command:?}"
    );
    drop(watch);
    assert!(
        receiver.recv_timeout(Duration::from_secs(3)).is_err(),
        "the stopped watch sends nothing more"
    );
}
