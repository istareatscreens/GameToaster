use std::collections::{BTreeMap, BTreeSet};

use windows::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows::Win32::System::Threading::{
    WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};

use super::handle::{CreationTime, ProcessHandle};
use super::snapshot::{snapshot, SnapshotEntry};
use super::ENDED_BY_LAUNCHER;

const UNKNOWN: CreationTime = 0;

pub struct Opened {
    pub handle: Option<ProcessHandle>,
    pub created: CreationTime,
}

struct Member {
    created: CreationTime,
    handle: Option<ProcessHandle>,

    gone_by: Option<CreationTime>,
}

impl Member {
    fn could_have_started(&self, created: CreationTime) -> bool {
        created >= self.created && self.gone_by.is_none_or(|gone| created <= gone)
    }
}

#[derive(Default)]
pub struct ProcessTree {
    members: BTreeMap<u32, Member>,

    running: BTreeSet<u32>,
}

impl ProcessTree {
    pub fn add_root(&mut self, pid: u32, created: CreationTime) {
        let handle = open_member(pid)
            .filter(|handle| created == UNKNOWN || handle.created() == Some(created));
        let created = match &handle {
            Some(handle) if created == UNKNOWN => handle.created().unwrap_or(UNKNOWN),
            _ => created,
        };
        let gone_by = handle.is_none().then(now);
        self.members.insert(
            pid,
            Member {
                created,
                handle,
                gone_by,
            },
        );
    }

    pub fn refresh_from(&mut self, entries: &[SnapshotEntry]) {
        self.absorb(entries, now(), |pid| {
            let handle = open_member(pid)?;
            let created = handle.created()?;
            Some(Opened {
                handle: Some(handle),
                created,
            })
        });
    }

    pub fn absorb(
        &mut self,
        entries: &[SnapshotEntry],
        taken: CreationTime,
        mut open: impl FnMut(u32) -> Option<Opened>,
    ) {
        self.running = entries.iter().map(|entry| entry.pid).collect();
        for (pid, member) in &mut self.members {
            if member.handle.is_none() && member.gone_by.is_none() && !self.running.contains(pid) {
                member.gone_by = Some(taken);
            }
        }
        loop {
            let mut joined = false;
            for entry in entries {
                if entry.pid == entry.parent || self.members.contains_key(&entry.pid) {
                    continue;
                }
                if !self.members.contains_key(&entry.parent) {
                    continue;
                }
                let Some(opened) = open(entry.pid) else {
                    continue;
                };
                if !self.members[&entry.parent].could_have_started(opened.created) {
                    continue;
                }
                self.members.insert(
                    entry.pid,
                    Member {
                        created: opened.created,
                        handle: opened.handle,
                        gone_by: None,
                    },
                );
                joined = true;
            }
            if !joined {
                break;
            }
        }
    }

    pub fn is_running_member(&self, pid: u32) -> bool {
        self.members
            .get(&pid)
            .is_some_and(|member| match &member.handle {
                Some(handle) => !handle.has_exited(),
                None => member.gone_by.is_none() && self.running.contains(&pid),
            })
    }

    #[cfg(test)]
    pub fn running_members(&self) -> Vec<u32> {
        self.members
            .iter()
            .filter(|(pid, member)| match &member.handle {
                Some(handle) => !handle.has_exited(),
                None => member.gone_by.is_none() && self.running.contains(pid),
            })
            .map(|(pid, _)| *pid)
            .collect()
    }
}

pub fn direct_children(parent: u32, parent_created: CreationTime) -> Vec<ProcessHandle> {
    snapshot()
        .into_iter()
        .filter(|entry| entry.parent == parent && entry.pid != parent)
        .filter_map(|entry| {
            ProcessHandle::open(
                entry.pid,
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            )
        })
        .filter(|handle| {
            handle
                .created()
                .is_some_and(|created| created >= parent_created)
        })
        .collect()
}

pub fn descendants(root: u32, root_created: CreationTime) -> Vec<ProcessHandle> {
    let entries = snapshot();
    let mut created: BTreeMap<u32, CreationTime> = BTreeMap::from([(root, root_created)]);
    let mut found = Vec::new();
    loop {
        let mut joined = false;
        for entry in &entries {
            if entry.pid == entry.parent || created.contains_key(&entry.pid) {
                continue;
            }
            let Some(&parent_created) = created.get(&entry.parent) else {
                continue;
            };
            let Some(handle) = ProcessHandle::open(
                entry.pid,
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            ) else {
                continue;
            };
            let Some(child_created) = handle.created() else {
                continue;
            };
            if child_created < parent_created {
                continue;
            }
            created.insert(entry.pid, child_created);
            found.push(handle);
            joined = true;
        }
        if !joined {
            return found;
        }
    }
}

pub fn end_tree(root: u32, root_created: CreationTime, end_root: impl FnOnce(), wait_ms: u32) {
    let mut started = descendants(root, root_created);
    end_root();
    for late in descendants(root, root_created) {
        if started.iter().all(|known| known.pid() != late.pid()) {
            started.push(late);
        }
    }
    for program in &started {
        program.terminate(ENDED_BY_LAUNCHER);
    }

    for program in &started {
        unsafe {
            let _ = WaitForSingleObject(program.raw(), wait_ms);
        }
    }
}

pub fn end_process_tree(root: u32, root_created: CreationTime, wait_ms: u32) {
    let Some(handle) = ProcessHandle::open(
        root,
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
    ) else {
        return;
    };
    if handle.created() != Some(root_created) {
        return;
    }
    end_tree(
        root,
        root_created,
        || {
            handle.terminate(ENDED_BY_LAUNCHER);
            unsafe {
                let _ = WaitForSingleObject(handle.raw(), wait_ms);
            }
        },
        wait_ms,
    );
}

fn open_member(pid: u32) -> Option<ProcessHandle> {
    ProcessHandle::open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE)
}

fn now() -> CreationTime {
    let time = unsafe { GetSystemTimeAsFileTime() };
    u64::from(time.dwHighDateTime) << 32 | u64::from(time.dwLowDateTime)
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/tree__tests.rs"]
mod tests;
