use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HANDLE;

use super::*;
use crate::support::process::handle::creation_time;
use crate::support::process::snapshot::snapshot;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const GAME: GameId = GameId(4);

fn system(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32")
        .join(name)
}

fn spawn(program: &Path, arguments: &str) -> Child {
    Command::new(program)
        .raw_arg(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .unwrap()
}

fn created(child: &Child) -> CreationTime {
    creation_time(HANDLE(child.as_raw_handle())).unwrap()
}

fn child_of(parent: u32) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(entry) = snapshot().into_iter().find(|entry| {
            entry.parent == parent && entry.pid != parent && entry.name != "conhost.exe"
        }) {
            return entry.pid;
        }
        assert!(Instant::now() < deadline, "{parent} started a program");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn renamed_ping(folder: &Path, name: &str) -> PathBuf {
    let copy = folder.join(name);
    std::fs::copy(system("PING.EXE"), &copy).unwrap();
    copy
}

fn end(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn end_tree(root: u32, created: CreationTime) {
    for handle in crate::support::process::tree::descendants(root, created) {
        handle.terminate(1);
    }
}

#[test]
fn a_program_started_for_a_game_and_what_it_starts_belong_to_its_session_until_it_ends() {
    let mut root = spawn(&system("cmd.exe"), "/d /c ping -n 30 127.0.0.1 >nul");
    let stranger = spawn(&system("PING.EXE"), "-n 30 127.0.0.1");
    let mut sessions = Sessions::default();
    sessions.follow_game(GAME, root.id(), created(&root));
    let ping = child_of(root.id());

    sessions.refresh(&snapshot());
    assert!(sessions.contains(root.id()));
    assert!(sessions.contains(ping), "the root's child joined");
    assert!(!sessions.contains(stranger.id()), "a stranger stays out");
    assert!(sessions.was_started(root.id()));
    assert!(
        !sessions.was_started(ping),
        "a descendant was not started by GameToaster"
    );

    sessions.end_game(GAME);
    assert!(!sessions.contains(ping));
    assert!(sessions.is_empty());
    end_tree(root.id(), created(&root));
    let _ = root.kill();
    let _ = root.wait();
    end(stranger);
}

#[test]
fn a_process_whose_parent_already_exited_still_belongs() {
    let mut root = spawn(
        &system("cmd.exe"),
        "/d /c start \"\" /b \"%SystemRoot%\\System32\\PING.EXE\" -n 30 127.0.0.1",
    );
    let root_pid = root.id();
    let root_created = created(&root);
    let mut sessions = Sessions::default();
    sessions.follow_game(GAME, root_pid, root_created);
    let ping = child_of(root_pid);
    let _ = root.wait();

    sessions.refresh(&snapshot());
    assert!(sessions.contains(ping), "a child of an exited root joins");
    end_tree(root_pid, root_created);
}

#[test]
fn the_watch_target_and_what_it_starts_belong_by_name_or_by_path() {
    let folder = tempfile::tempdir().unwrap();
    let target = renamed_ping(folder.path(), "gt-watch-target-5e1c.exe");
    let other_folder = tempfile::tempdir().unwrap();
    let namesake = renamed_ping(other_folder.path(), "gt-watch-target-5e1c.exe");
    let game = spawn(&target, "-n 30 127.0.0.1");
    let lookalike = spawn(&namesake, "-n 30 127.0.0.1");

    let mut by_name = Sessions::default();
    by_name.follow_target(GAME, "gt-watch-target-5e1c.exe", None);
    by_name.refresh(&snapshot());
    assert!(by_name.contains(game.id()));
    assert!(by_name.contains(lookalike.id()));
    assert!(
        !by_name.was_started(game.id()),
        "a watch-target match was not started by GameToaster"
    );

    let mut by_path = Sessions::default();
    by_path.follow_target(GAME, "gt-watch-target-5e1c.exe", Some(&target));
    by_path.refresh(&snapshot());
    assert!(by_path.contains(game.id()));
    assert!(!by_path.contains(lookalike.id()));

    end(game);
    end(lookalike);
}

#[test]
fn an_empty_watch_target_follows_nothing_and_a_repeated_one_is_kept_once() {
    let mut sessions = Sessions::default();
    sessions.follow_target(GAME, "   ", None);
    assert!(sessions.is_empty());
    sessions.follow_target(GAME, "game.exe", None);
    sessions.follow_target(GAME, "GAME.EXE", None);
    assert_eq!(sessions.games[&GAME].targets.len(), 1);
}

#[test]
fn a_close_script_is_followed_for_a_while_only() {
    let mut script = spawn(&system("PING.EXE"), "-n 30 127.0.0.1");
    let mut sessions = Sessions::default();
    let now = Instant::now();
    sessions.follow_transient(script.id(), created(&script), now);
    assert!(sessions.contains(script.id()));
    assert!(sessions.was_started(script.id()));

    sessions.expire(now + TRANSIENT_WINDOW - Duration::from_secs(1));
    assert!(sessions.contains(script.id()), "still within its time");
    sessions.expire(now + TRANSIENT_WINDOW);
    assert!(!sessions.contains(script.id()), "its time has passed");
    assert!(sessions.is_empty());
    let _ = script.kill();
    let _ = script.wait();
}
