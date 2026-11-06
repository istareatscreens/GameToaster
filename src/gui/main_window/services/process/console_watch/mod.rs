mod console_windows;
mod decide;
mod sessions;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::{
    GetCurrentThread, GetCurrentThreadId, SetThreadPriority, INFINITE, THREAD_PRIORITY_BELOW_NORMAL,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, PostThreadMessageW,
    TranslateMessage, CHILDID_SELF, EVENT_OBJECT_SHOW, MSG, MWMO_INPUTAVAILABLE, OBJID_WINDOW,
    PM_NOREMOVE, PM_REMOVE, QS_ALLINPUT, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP,
    WM_QUIT, WM_USER,
};

use crate::launcher_gui::model::GameId;
use crate::support::process::handle::{creation_time_of, CreationTime};
use crate::support::process::pause::PAUSED_TITLE;
use crate::support::process::snapshot::snapshot;
use console_windows::ConsoleWindow;
use decide::{Action, Decisions, Seen};
use sessions::Sessions;

const FAST_PASS: Duration = Duration::from_millis(250);

const SLOW_PASS: Duration = Duration::from_secs(1);

const FAST_PHASE: Duration = Duration::from_secs(45);

pub struct ConsoleWatch {
    handle: ConsoleWatchHandle,
    thread: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct ConsoleWatchHandle {
    commands: Sender<Command>,
    thread_id: u32,
}

#[derive(Debug)]
pub(crate) enum Command {
    FollowGame {
        game: GameId,
        pid: u32,
        created: CreationTime,
    },
    FollowTarget {
        game: GameId,
        name: String,
        path: Option<PathBuf>,
    },
    FollowTransient {
        pid: u32,
        created: CreationTime,
    },
    EndGame(GameId),
    SetHiding(bool),
}

impl ConsoleWatch {
    pub fn start(hiding: bool) -> Result<Self, String> {
        let (commands, receiver) = mpsc::channel();
        let (ready_sender, ready) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("gametoaster-console-watch".into())
            .spawn(move || run(receiver, hiding, ready_sender))
            .map_err(|error| format!("failed to start the console watcher: {error}"))?;
        let thread_id = ready
            .recv()
            .map_err(|_| "the console watcher did not start".to_string())?;
        Ok(Self {
            handle: ConsoleWatchHandle {
                commands,
                thread_id,
            },
            thread: Some(thread),
        })
    }

    pub fn handle(&self) -> ConsoleWatchHandle {
        self.handle.clone()
    }
}

impl std::ops::Deref for ConsoleWatch {
    type Target = ConsoleWatchHandle;

    fn deref(&self) -> &ConsoleWatchHandle {
        &self.handle
    }
}

impl ConsoleWatchHandle {
    pub fn follow_game(&self, game: GameId, pid: u32, created: CreationTime) {
        self.send(Command::FollowGame { game, pid, created });
    }

    pub fn follow_watch_target(&self, game: GameId, name: &str, path: Option<PathBuf>) {
        self.send(Command::FollowTarget {
            game,
            name: name.to_string(),
            path,
        });
    }

    pub fn follow_transient(&self, pid: u32, created: CreationTime) {
        self.send(Command::FollowTransient { pid, created });
    }

    pub fn end_game(&self, game: GameId) {
        self.send(Command::EndGame(game));
    }

    pub fn set_hiding(&self, hiding: bool) {
        self.send(Command::SetHiding(hiding));
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_ok() {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_APP, WPARAM(0), LPARAM(0));
            }
        }
    }
}

#[cfg(test)]
impl ConsoleWatchHandle {
    pub(crate) fn recording() -> (Self, Receiver<Command>) {
        let (commands, receiver) = mpsc::channel();
        (
            Self {
                commands,
                thread_id: 0,
            },
            receiver,
        )
    }
}

#[cfg(test)]
pub(crate) fn followed(receiver: &Receiver<Command>) -> Vec<(Option<GameId>, u32)> {
    receiver
        .try_iter()
        .filter_map(|command| match command {
            Command::FollowGame { game, pid, .. } => Some((Some(game), pid)),
            Command::FollowTransient { pid, .. } => Some((None, pid)),
            _ => None,
        })
        .collect()
}

impl Drop for ConsoleWatch {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.handle.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

thread_local! {

    static SHOWN: RefCell<Vec<isize>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "system" fn on_shown(
    _hook: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    if event == EVENT_OBJECT_SHOW
        && object == OBJID_WINDOW.0
        && child == CHILDID_SELF as i32
        && !window.0.is_null()
    {
        SHOWN.with(|shown| shown.borrow_mut().push(window.0 as isize));
    }
}

fn run(commands: Receiver<Command>, hiding: bool, ready: SyncSender<u32>) {
    unsafe {
        let mut message = MSG::default();
        let _ = PeekMessageW(&mut message, None, WM_USER, WM_USER, PM_NOREMOVE);
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
    if ready.send(unsafe { GetCurrentThreadId() }).is_err() {
        return;
    }
    let mut watcher = Watcher::new(hiding);
    loop {
        watcher.keep_hook();
        let wait = watcher.wait(Instant::now());
        unsafe {
            MsgWaitForMultipleObjectsEx(None, wait, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            if message.message == WM_QUIT {
                watcher.unhook();
                return;
            }
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        loop {
            match commands.try_recv() {
                Ok(command) => watcher.apply(command, Instant::now()),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    watcher.unhook();
                    return;
                }
            }
        }
        let shown = SHOWN.with(|shown| std::mem::take(&mut *shown.borrow_mut()));
        watcher.on_shown(shown);
        watcher.pass_if_due(Instant::now());
    }
}

struct Watcher {
    hiding: bool,
    sessions: Sessions,
    decisions: Decisions,
    hook: Option<HWINEVENTHOOK>,

    fast_until: Instant,
    next_pass: Instant,

    strangers: BTreeSet<(u32, CreationTime)>,
}

impl Watcher {
    fn new(hiding: bool) -> Self {
        let now = Instant::now();
        Self {
            hiding,
            sessions: Sessions::default(),
            decisions: Decisions::default(),
            hook: None,
            fast_until: now,
            next_pass: now,
            strangers: BTreeSet::new(),
        }
    }

    fn active(&self) -> bool {
        self.hiding && !self.sessions.is_empty()
    }

    fn keep_hook(&mut self) {
        match (self.active(), self.hook) {
            (true, None) => {
                let hook = unsafe {
                    SetWinEventHook(
                        EVENT_OBJECT_SHOW,
                        EVENT_OBJECT_SHOW,
                        HMODULE::default(),
                        Some(on_shown),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    )
                };

                self.hook = (!hook.0.is_null()).then_some(hook);
            }
            (false, Some(_)) => self.unhook(),
            _ => {}
        }
    }

    fn unhook(&mut self) {
        if let Some(hook) = self.hook.take() {
            unsafe {
                let _ = UnhookWinEvent(hook);
            }
        }
    }

    fn wait(&self, now: Instant) -> u32 {
        if !self.active() {
            return INFINITE;
        }
        let until = self.next_pass.saturating_duration_since(now).as_millis();
        u32::try_from(until).unwrap_or(INFINITE - 1)
    }

    fn apply(&mut self, command: Command, now: Instant) {
        match command {
            Command::FollowGame { game, pid, created } => {
                self.sessions.follow_game(game, pid, created);
                self.followed(now);
            }
            Command::FollowTarget { game, name, path } => {
                self.sessions.follow_target(game, &name, path.as_deref());
                self.followed(now);
            }
            Command::FollowTransient { pid, created } => {
                self.sessions.follow_transient(pid, created, now);
                self.followed(now);
            }
            Command::EndGame(game) => self.sessions.end_game(game),
            Command::SetHiding(hiding) => {
                if hiding == self.hiding {
                    return;
                }
                self.hiding = hiding;
                if hiding {
                    self.next_pass = now;
                } else {
                    self.show_hidden();
                }
            }
        }
    }

    fn followed(&mut self, now: Instant) {
        self.fast_until = now + FAST_PHASE;
        self.next_pass = now;
        self.strangers.clear();
    }

    fn on_shown(&mut self, windows: Vec<isize>) {
        if !self.active() || windows.is_empty() {
            return;
        }
        let consoles: Vec<ConsoleWindow> = windows
            .into_iter()
            .filter_map(console_windows::look)
            .collect();
        self.consider(&consoles, false);
    }

    fn pass_if_due(&mut self, now: Instant) {
        if !self.active() || now < self.next_pass {
            return;
        }
        self.sessions.expire(now);
        let fast = now < self.fast_until;
        if fast {
            self.sessions.refresh(&snapshot());
        }
        self.consider(&console_windows::all(), fast);
        self.decisions.forget_gone(console_windows::exists);
        self.next_pass = now + if fast { FAST_PASS } else { SLOW_PASS };
    }

    fn consider(&mut self, consoles: &[ConsoleWindow], refreshed: bool) {
        let owners: Vec<(u32, Option<CreationTime>)> = consoles
            .iter()
            .map(|console| (console.owner, creation_time_of(console.owner)))
            .collect();
        let unknown = owners.iter().any(|(owner, created)| {
            !self.sessions.contains(*owner)
                && created.is_some_and(|created| !self.strangers.contains(&(*owner, created)))
        });
        if unknown && !refreshed {
            self.sessions.refresh(&snapshot());
        }
        for (console, (owner, created)) in consoles.iter().zip(owners) {
            if !self.sessions.contains(owner) {
                if let Some(created) = created {
                    self.strangers.insert((owner, created));
                }
                continue;
            }
            match self.decisions.member(&seen(console), self.hiding) {
                Action::Hide => console_windows::hide(console.window),
                Action::Reveal => console_windows::reveal(console.window),
                Action::Show { minimized } => console_windows::show(console.window, minimized),
                Action::Nothing => {}
            }
        }
    }

    fn show_hidden(&mut self) {
        for (window, minimized) in self.decisions.stop_hiding(console_windows::exists) {
            console_windows::show(window, minimized);
        }
        if self.sessions.is_empty() {
            return;
        }
        self.sessions.refresh(&snapshot());
        for console in console_windows::all() {
            if !self.sessions.was_started(console.owner) {
                continue;
            }
            if let Action::Show { minimized } = self.decisions.started_hidden(&seen(&console)) {
                console_windows::show(console.window, minimized);
            }
        }
    }
}

fn seen(console: &ConsoleWindow) -> Seen {
    Seen {
        window: console.window,
        visible: console.visible,
        minimized: console.minimized,
        paused: console.title.trim() == PAUSED_TITLE,
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/services/process/console_watch/mod__tests.rs"]
mod tests;
