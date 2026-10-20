use std::collections::{BTreeMap, BTreeSet};

use windows::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows::Win32::System::Threading::{
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};

use super::handle::{CreationTime, ProcessHandle};
use super::snapshot::{snapshot, SnapshotEntry};

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

    pub fn refresh(&mut self) {
        let entries = snapshot();
        self.absorb(&entries, now(), |pid| {
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
