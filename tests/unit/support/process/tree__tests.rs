use std::collections::BTreeMap;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HANDLE;

use super::*;
use crate::support::process::handle::creation_time;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn joined(tree: &ProcessTree, pid: u32) -> bool {
    tree.members.contains_key(&pid)
}

fn entry(pid: u32, parent: u32) -> SnapshotEntry {
    SnapshotEntry {
        pid,
        parent,
        name: String::new(),
    }
}

fn tree_with_root(pid: u32, created: CreationTime) -> ProcessTree {
    let mut tree = ProcessTree::default();
    tree.members.insert(
        pid,
        Member {
            created,
            handle: None,
            gone_by: None,
        },
    );
    tree
}

fn opener(created: &BTreeMap<u32, CreationTime>) -> impl FnMut(u32) -> Option<Opened> + '_ {
    move |pid| {
        created.get(&pid).map(|&created| Opened {
            handle: None,
            created,
        })
    }
}

#[test]
fn children_and_grandchildren_join_in_any_snapshot_order() {
    let mut tree = tree_with_root(100, 10);
    let created = BTreeMap::from([(200, 20), (300, 30), (400, 40)]);
    tree.absorb(
        &[
            entry(300, 200),
            entry(100, 1),
            entry(200, 100),
            entry(400, 999),
        ],
        50,
        opener(&created),
    );
    assert!(joined(&tree, 200) && joined(&tree, 300));
    assert!(!joined(&tree, 400), "a stranger does not join");
    assert_eq!(tree.running_members(), [100, 200, 300]);
}

#[test]
fn a_process_older_than_its_parent_member_is_not_its_child() {
    let mut tree = tree_with_root(100, 50);
    tree.absorb(
        &[entry(100, 1), entry(200, 100)],
        60,
        opener(&BTreeMap::from([(200, 40)])),
    );
    assert!(!joined(&tree, 200), "its parent id named an older process");
}

#[test]
fn the_children_of_an_exited_member_still_join() {
    let mut tree = tree_with_root(100, 10);
    let created = BTreeMap::from([(200, 20), (300, 30)]);
    tree.absorb(&[entry(100, 1), entry(200, 100)], 25, opener(&created));
    assert!(joined(&tree, 200));

    tree.absorb(&[entry(100, 1), entry(300, 200)], 60, opener(&created));
    assert!(joined(&tree, 300));
    assert_eq!(tree.running_members(), [100, 300]);
}

#[test]
fn a_process_reusing_the_id_of_a_gone_member_is_not_that_member() {
    let mut tree = tree_with_root(100, 10);

    tree.absorb(&[entry(7, 1)], 20, opener(&BTreeMap::new()));

    tree.absorb(
        &[entry(100, 5), entry(200, 100)],
        40,
        opener(&BTreeMap::from([(100, 25), (200, 30)])),
    );
    assert!(!joined(&tree, 200), "created after the root was gone");
    assert!(
        tree.running_members().is_empty(),
        "the id's new process is not the root"
    );
}

#[test]
fn a_process_that_cannot_be_opened_does_not_join() {
    let mut tree = tree_with_root(100, 10);
    tree.absorb(
        &[entry(100, 1), entry(200, 100)],
        50,
        opener(&BTreeMap::new()),
    );
    assert!(!joined(&tree, 200));
}

fn spawn_hidden(program: &PathBuf, arguments: &[&str]) -> Child {
    Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .unwrap()
}

#[test]
fn a_real_process_tree_is_followed_and_a_stranger_is_left_out() {
    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    let mut root = spawn_hidden(
        &system.join("cmd.exe"),
        &["/d", "/c", "ping -n 30 127.0.0.1 >nul"],
    );
    let mut stranger = spawn_hidden(&system.join("PING.EXE"), &["-n", "30", "127.0.0.1"]);
    let root_created = creation_time(HANDLE(root.as_raw_handle())).unwrap();
    let mut tree = ProcessTree::default();
    tree.add_root(root.id(), root_created);

    let children_of_root = || {
        let mut children: Vec<u32> = snapshot()
            .into_iter()
            .filter(|entry| entry.parent == root.id() && entry.pid != root.id())
            .map(|entry| entry.pid)
            .collect();
        children.sort_unstable();
        children
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    let expected = loop {
        let first = children_of_root();
        std::thread::sleep(Duration::from_millis(300));
        if !first.is_empty() && first == children_of_root() {
            break first;
        }
        assert!(
            Instant::now() < deadline,
            "the root's children never settled"
        );
    };

    tree.refresh();
    for pid in &expected {
        assert!(joined(&tree, *pid), "child {pid} of the root joined");
    }
    assert!(!joined(&tree, stranger.id()), "the stranger stays out");
    let children = direct_children(root.id(), root_created);
    let mut found: Vec<u32> = children.iter().map(ProcessHandle::pid).collect();
    found.sort_unstable();
    assert_eq!(found, expected);

    for handle in &children {
        assert!(handle.terminate(1));
    }
    let _ = root.wait();
    let _ = stranger.kill();
    let _ = stranger.wait();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !tree.running_members().is_empty() {
        assert!(Instant::now() < deadline, "every member ended");
        tree.refresh();
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_root_reported_without_a_creation_time_takes_its_own_and_is_followed() {
    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    let mut root = spawn_hidden(&system.join("PING.EXE"), &["-n", "30", "127.0.0.1"]);
    let real = creation_time(HANDLE(root.as_raw_handle())).unwrap();
    let mut tree = ProcessTree::default();
    tree.add_root(root.id(), UNKNOWN);

    let member = &tree.members[&root.id()];
    assert_eq!(member.created, real);
    assert!(member.handle.is_some() && member.gone_by.is_none());
    assert_eq!(tree.running_members(), vec![root.id()]);

    let _ = root.kill();
    let _ = root.wait();
}
