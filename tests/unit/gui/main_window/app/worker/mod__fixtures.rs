use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;

use super::context::WorkerContext;
use super::messages::{WorkerCommand, WorkerResult};
use crate::gui::common::game_state::GameStateHub;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::{GameEntry, GameId, LauncherConfig, LauncherSettings};

const PATIENCE: Duration = Duration::from_secs(20);

pub(super) fn released_handoff() -> crate::launcher_gui::runtime::HandoffGate {
    let gate = crate::launcher_gui::runtime::HandoffGate::new();
    gate.answer(Ok(()));
    gate
}

pub(super) struct Harness {
    pub(super) context: WorkerContext,
    pub(super) results: Receiver<WorkerResult>,
    commands: Receiver<WorkerCommand>,

    folder: tempfile::TempDir,

    _state: tempfile::TempDir,
}

impl Harness {
    pub(super) fn new() -> Self {
        Self::with_watch(None)
    }

    pub(super) fn watched() -> (Self, Receiver<crate::launcher_gui::process::WatchCommand>) {
        let (handle, receiver) = crate::launcher_gui::process::ConsoleWatchHandle::recording();
        (Self::with_watch(Some(handle)), receiver)
    }

    fn with_watch(watch: Option<crate::launcher_gui::process::ConsoleWatchHandle>) -> Self {
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let (results_sender, results) = mpsc::channel();
        let (commands_sender, commands) = mpsc::sync_channel(64);
        let context = WorkerContext::new(
            state.path().to_path_buf(),
            LauncherLog::open(state.path()).unwrap(),
            egui::Context::default(),
            GameStateHub::default(),
            results_sender,
            commands_sender,
            watch,
        );
        Self {
            context,
            results,
            commands,
            folder,
            _state: state,
        }
    }

    pub(super) fn marks_file(&self) -> PathBuf {
        self.folder.path().join("marks.txt")
    }

    pub(super) fn script(&self, name: &str, mark: &str, rest: &[&str], code: i32) -> PathBuf {
        let path = self.folder.path().join(name);
        let mut text = format!(
            "@echo off\r\n>>\"{}\" echo {mark}\r\n",
            self.marks_file().display()
        );
        for line in rest {
            text.push_str(line);
            text.push_str("\r\n");
        }
        text.push_str(&format!("exit /b {code}\r\n"));
        fs::write(&path, text).unwrap();
        path
    }

    pub(super) fn lasting_game(&self, name: &str, mark: &str) -> PathBuf {
        self.script(name, mark, &["ping -n 60 127.0.0.1 >nul"], 0)
    }

    pub(super) fn absent(&self, name: &str) -> PathBuf {
        self.folder.path().join(name)
    }

    pub(super) fn marks(&self) -> Vec<String> {
        fs::read_to_string(self.marks_file())
            .map(|text| {
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn wait_for_marks(&mut self, expected: &[&str]) -> Vec<String> {
        let deadline = Instant::now() + PATIENCE;
        loop {
            self.pump();
            let marks = self.marks();
            if marks == expected || Instant::now() >= deadline {
                return marks;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub(super) fn marks_after(&mut self, time: Duration) -> Vec<String> {
        let deadline = Instant::now() + time;
        while Instant::now() < deadline {
            self.pump();
            std::thread::sleep(Duration::from_millis(100));
        }
        self.marks()
    }

    pub(super) fn pump(&mut self) {
        while let Ok(command) = self.commands.try_recv() {
            match command {
                WorkerCommand::ProcessCompletion(completion) => {
                    let _ = self.context.process_completion(completion);
                }
                WorkerCommand::WatchEvent(event) => {
                    let _ = self.context.watch_event(event);
                }
                WorkerCommand::PluginRuntimeEvent {
                    game_id,
                    operation_id,
                    event,
                } => {
                    let _ = self
                        .context
                        .plugin_runtime_event(game_id, operation_id, event);
                }
                _ => {}
            }
        }
        self.context.emit_completed_games();
    }

    pub(super) fn posted(&self) -> Vec<WorkerResult> {
        self.results.try_iter().collect()
    }

    pub(super) fn pump_until(
        &mut self,
        wanted: impl Fn(&WorkerResult) -> bool,
    ) -> Vec<WorkerResult> {
        let deadline = Instant::now() + PATIENCE;
        let mut posted = Vec::new();
        while Instant::now() < deadline {
            self.pump();
            posted.extend(self.results.try_iter());
            if posted.iter().any(&wanted) {
                return posted;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("the expected result was not posted: {posted:?}");
    }
}

pub(super) fn game(target: &Path) -> GameEntry {
    GameEntry {
        id: GameId(1),
        label: "Game".into(),
        controller_image: None,
        selected_target: target.to_path_buf(),
        plugin: None,
        hotkey: None,
        watch_exe: None,
        pre_start_script: None,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    }
}

pub(super) fn config(game: &GameEntry) -> LauncherConfig {
    LauncherConfig {
        settings: LauncherSettings {
            show_command_windows: false,
            pause_on_failure: false,
            ..LauncherSettings::default()
        },
        default_game: None,
        next_game_id: game.id.0 + 1,
        games: vec![game.clone()],
    }
}
