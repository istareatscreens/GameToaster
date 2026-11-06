use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::super::discovery::{process_name_matches, process_path_matches};
use crate::launcher_gui::model::GameId;
use crate::support::process::handle::{creation_time_of, image_path_of, CreationTime};
use crate::support::process::snapshot::SnapshotEntry;
use crate::support::process::tree::ProcessTree;

pub(super) const TRANSIENT_WINDOW: Duration = Duration::from_secs(45);

#[derive(Debug)]
struct Target {
    name: String,
    path: Option<PathBuf>,
}

impl Target {
    fn matches(&self, entry: &SnapshotEntry) -> bool {
        match &self.path {
            Some(path) => {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                process_name_matches(&entry.name, name)
                    && image_path_of(entry.pid)
                        .is_some_and(|image| process_path_matches(&image, path))
            }
            None => process_name_matches(&entry.name, &self.name),
        }
    }
}

#[derive(Default)]
struct Game {
    tree: ProcessTree,

    started: BTreeSet<u32>,

    matched: BTreeSet<u32>,
    targets: Vec<Target>,
}

struct Transient {
    tree: ProcessTree,
    root: u32,
    until: Instant,
}

#[derive(Default)]
pub(super) struct Sessions {
    games: BTreeMap<GameId, Game>,
    transients: Vec<Transient>,
}

impl Sessions {
    pub fn follow_game(&mut self, game: GameId, pid: u32, created: CreationTime) {
        let session = self.games.entry(game).or_default();
        session.tree.add_root(pid, created);
        session.started.insert(pid);
    }

    pub fn follow_target(&mut self, game: GameId, name: &str, path: Option<&Path>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let session = self.games.entry(game).or_default();
        let known = session
            .targets
            .iter()
            .any(|target| target.name.eq_ignore_ascii_case(name) && target.path.as_deref() == path);
        if !known {
            session.targets.push(Target {
                name: name.to_string(),
                path: path.map(Path::to_path_buf),
            });
        }
    }

    pub fn follow_transient(&mut self, pid: u32, created: CreationTime, now: Instant) {
        let mut tree = ProcessTree::default();
        tree.add_root(pid, created);
        self.transients.push(Transient {
            tree,
            root: pid,
            until: now + TRANSIENT_WINDOW,
        });
    }

    pub fn end_game(&mut self, game: GameId) {
        self.games.remove(&game);
    }

    pub fn expire(&mut self, now: Instant) {
        self.transients.retain(|transient| transient.until > now);
    }

    pub fn is_empty(&self) -> bool {
        self.games.is_empty() && self.transients.is_empty()
    }

    pub fn refresh(&mut self, entries: &[SnapshotEntry]) {
        for session in self.games.values_mut() {
            for entry in entries {
                if session.started.contains(&entry.pid)
                    || session.matched.contains(&entry.pid)
                    || !session.targets.iter().any(|target| target.matches(entry))
                {
                    continue;
                }
                if let Some(created) = creation_time_of(entry.pid) {
                    session.tree.add_root(entry.pid, created);
                    session.matched.insert(entry.pid);
                }
            }
            session.tree.refresh_from(entries);
        }
        for transient in &mut self.transients {
            transient.tree.refresh_from(entries);
        }
    }

    pub fn contains(&self, pid: u32) -> bool {
        self.games
            .values()
            .any(|session| session.tree.is_running_member(pid))
            || self
                .transients
                .iter()
                .any(|transient| transient.tree.is_running_member(pid))
    }

    pub fn was_started(&self, pid: u32) -> bool {
        self.games
            .values()
            .any(|session| session.started.contains(&pid))
            || self
                .transients
                .iter()
                .any(|transient| transient.root == pid)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/services/process/console_watch/sessions__tests.rs"]
mod tests;
