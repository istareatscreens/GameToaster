use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use crate::launcher_gui::process;
use crate::support::process::console_policy::ConsoleSettings;
use crate::support::process::handle::{creation_time_of, CreationTime};
use crate::support::process::tree::ProcessTree;

const WATCH_WINDOW: Duration = Duration::from_secs(45);

const PASS_INTERVAL: Duration = Duration::from_millis(250);

pub(crate) fn follow_root(
    hider: &mut Option<ConsoleHider>,
    settings: ConsoleSettings,
    pid: u32,
    created: CreationTime,
) {
    if settings.hides_consoles() {
        hider
            .get_or_insert_with(ConsoleHider::new)
            .add_root(pid, created);
    }
}

pub(crate) fn follow_watch_target(
    hider: &mut Option<ConsoleHider>,
    settings: ConsoleSettings,
    name: &str,
) {
    if settings.hides_consoles() {
        hider
            .get_or_insert_with(ConsoleHider::new)
            .watch_target(name);
    }
}

pub(crate) fn tick(hider: &mut Option<ConsoleHider>, settings: ConsoleSettings) -> bool {
    if !settings.hides_consoles() {
        *hider = None;
    }
    let running = hider.as_mut().is_some_and(ConsoleHider::tick);
    if !running {
        *hider = None;
    }
    running
}

pub(crate) struct ConsoleHider {
    tree: ProcessTree,
    watch_targets: Vec<String>,

    handled: BTreeSet<(u32, CreationTime)>,
    deadline: Instant,
    next_pass: Instant,
}

impl ConsoleHider {
    fn new() -> Self {
        Self {
            tree: ProcessTree::default(),
            watch_targets: Vec::new(),
            handled: BTreeSet::new(),
            deadline: Instant::now() + WATCH_WINDOW,
            next_pass: Instant::now(),
        }
    }

    fn add_root(&mut self, pid: u32, created: CreationTime) {
        self.tree.add_root(pid, created);
        self.extend();
    }

    fn watch_target(&mut self, name: &str) {
        let name = name.trim();
        if !name.is_empty()
            && !self
                .watch_targets
                .iter()
                .any(|known| known.eq_ignore_ascii_case(name))
        {
            self.watch_targets.push(name.to_string());
        }
        self.extend();
    }

    fn extend(&mut self) {
        self.deadline = self.deadline.max(Instant::now() + WATCH_WINDOW);
    }

    fn tick(&mut self) -> bool {
        let now = Instant::now();
        if now >= self.deadline {
            return false;
        }
        if now < self.next_pass {
            return true;
        }
        self.next_pass = now + PASS_INTERVAL;
        self.tree.refresh();
        let watched: Vec<u32> = self
            .watch_targets
            .iter()
            .flat_map(|name| process::pids_by_name(name))
            .collect();
        for pid in self.tree.running_members().into_iter().chain(watched) {
            let Some(created) = creation_time_of(pid) else {
                continue;
            };
            if !self.handled.contains(&(pid, created)) && process::hide_console_for_pid(pid) {
                self.handled.insert((pid, created));
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/console_hider__tests.rs"]
mod tests;
