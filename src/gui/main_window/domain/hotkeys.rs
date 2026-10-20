use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use eframe::egui;
use windows_hotkeys::keys::{ModKey, VKey};
use windows_hotkeys::singlethreaded::HotkeyManager;
use windows_hotkeys::{HotkeyManagerImpl, InterruptHandle};

use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::{GameId, LauncherConfig};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: u16,
}

impl Hotkey {
    pub fn from_egui(modifiers: egui::Modifiers, key: egui::Key) -> Option<Self> {
        Some(Self {
            ctrl: modifiers.ctrl || modifiers.command,
            alt: modifiers.alt,
            shift: modifiers.shift,
            win: false,
            key: vk_from_egui(key)?,
        })
    }

    pub fn has_modifier(self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }

    fn is_function_key(self) -> bool {
        (0x70..=0x87).contains(&self.key)
    }

    pub fn is_valid(self) -> bool {
        self.key != 0 && (self.has_modifier() || self.is_function_key())
    }

    fn modifiers(self) -> Vec<ModKey> {
        let mut keys = Vec::new();
        if self.ctrl {
            keys.push(ModKey::Ctrl);
        }
        if self.alt {
            keys.push(ModKey::Alt);
        }
        if self.shift {
            keys.push(ModKey::Shift);
        }
        if self.win {
            keys.push(ModKey::Win);
        }
        keys
    }

    fn trigger(self) -> VKey {
        VKey::CustomKeyCode(self.key as i32)
    }
}

impl std::fmt::Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".into());
        }
        if self.alt {
            parts.push("Alt".into());
        }
        if self.shift {
            parts.push("Shift".into());
        }
        if self.win {
            parts.push("Win".into());
        }
        parts.push(vk_name(self.key));
        write!(f, "{}", parts.join("+"))
    }
}

impl std::str::FromStr for Hotkey {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut hotkey = Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            key: 0,
        };
        for raw in value.split('+') {
            let token = raw.trim();
            if token.is_empty() {
                continue;
            }
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => hotkey.ctrl = true,
                "alt" | "menu" => hotkey.alt = true,
                "shift" => hotkey.shift = true,
                "win" | "super" | "meta" => hotkey.win = true,
                _ => {
                    hotkey.key = vk_from_name(token)
                        .ok_or_else(|| format!("unknown hotkey key '{token}'"))?
                }
            }
        }
        if hotkey.key == 0 {
            return Err(format!("hotkey '{value}' has no trigger key"));
        }
        Ok(hotkey)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HotkeyAction {
    OpenLauncher,

    CloseActiveGame,
    Launch(GameId),
    Close(GameId),
}

enum HotkeyCommand {
    Reload(Vec<(Hotkey, HotkeyAction)>),

    Shutdown,
}

pub struct HotkeyService {
    receiver: Receiver<HotkeyAction>,

    control: Sender<HotkeyCommand>,

    interrupt: Option<InterruptHandle>,
    thread: Option<JoinHandle<()>>,
}

impl HotkeyService {
    pub fn new(context: egui::Context, log: LauncherLog) -> Self {
        let (sender, receiver) = mpsc::channel();
        let (control_tx, control_rx) = mpsc::channel::<HotkeyCommand>();
        let (handle_tx, handle_rx) = mpsc::channel::<InterruptHandle>();
        let thread = std::thread::Builder::new()
            .name("gametoaster-hotkeys".into())
            .spawn(move || hotkey_listener(context, sender, log, control_rx, handle_tx))
            .expect("failed to spawn hotkey listener thread");

        let interrupt = handle_rx.recv().ok();
        Self {
            receiver,
            control: control_tx,
            interrupt,
            thread: Some(thread),
        }
    }

    pub fn try_recv(&self) -> Option<HotkeyAction> {
        self.receiver.try_recv().ok()
    }

    pub fn reload(&mut self, config: &LauncherConfig) {
        self.apply(Self::bindings_for(config));
    }

    pub fn suspend(&mut self) {
        self.apply(Vec::new());
        while self.receiver.try_recv().is_ok() {}
    }

    fn apply(&mut self, bindings: Vec<(Hotkey, HotkeyAction)>) {
        if self.control.send(HotkeyCommand::Reload(bindings)).is_ok() {
            if let Some(interrupt) = &self.interrupt {
                interrupt.interrupt();
            }
        }
    }

    fn bindings_for(config: &LauncherConfig) -> Vec<(Hotkey, HotkeyAction)> {
        let mut bindings: Vec<(Hotkey, HotkeyAction)> = Vec::new();
        if let Some(hotkey) = config.settings.launcher_hotkey {
            if hotkey.is_valid() {
                bindings.push((hotkey, HotkeyAction::OpenLauncher));
            }
        }
        if let Some(hotkey) = config.settings.close_game_hotkey {
            if hotkey.is_valid() {
                bindings.push((hotkey, HotkeyAction::CloseActiveGame));
            }
        }
        for game in &config.games {
            if let Some(hotkey) = game.hotkey {
                if hotkey.is_valid() {
                    bindings.push((hotkey, HotkeyAction::Launch(game.id)));
                }
            }
            if let Some(hotkey) = game.close_hotkey {
                if hotkey.is_valid() {
                    bindings.push((hotkey, HotkeyAction::Close(game.id)));
                }
            }
        }
        bindings
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        let _ = self.control.send(HotkeyCommand::Shutdown);
        if let Some(interrupt) = self.interrupt.take() {
            interrupt.interrupt();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn hotkey_listener(
    context: egui::Context,
    sender: Sender<HotkeyAction>,
    log: LauncherLog,
    control: Receiver<HotkeyCommand>,
    handle_tx: Sender<InterruptHandle>,
) {
    let mut manager: HotkeyManager<()> = HotkeyManager::new();

    let _ = handle_tx.send(manager.interrupt_handle());

    let mut bindings: Vec<(Hotkey, HotkeyAction)> = Vec::new();
    loop {
        if let Err(error) = manager.unregister_all() {
            log.event(
                "hotkey-unregister",
                &[("result", "failed".into()), ("error", error.to_string())],
            );
        }
        for (hotkey, action) in &bindings {
            let tx = sender.clone();
            let wake = context.clone();
            let action = *action;
            let registered = manager.register(hotkey.trigger(), &hotkey.modifiers(), move || {
                let _ = tx.send(action);

                wake.request_repaint();
            });
            if let Err(error) = registered {
                log.event(
                    "hotkey-register",
                    &[
                        ("binding", hotkey.to_string()),
                        ("action", format!("{action:?}")),
                        ("result", "failed".into()),
                        ("error", error.to_string()),
                    ],
                );
            }
        }

        manager.event_loop();
        match control.recv() {
            Ok(HotkeyCommand::Reload(new_bindings)) => bindings = new_bindings,
            Ok(HotkeyCommand::Shutdown) | Err(_) => break,
        }
    }
}

fn vk_from_egui(key: egui::Key) -> Option<u16> {
    use egui::Key::*;
    let code = match key {
        A => 0x41,
        B => 0x42,
        C => 0x43,
        D => 0x44,
        E => 0x45,
        F => 0x46,
        G => 0x47,
        H => 0x48,
        I => 0x49,
        J => 0x4A,
        K => 0x4B,
        L => 0x4C,
        M => 0x4D,
        N => 0x4E,
        O => 0x4F,
        P => 0x50,
        Q => 0x51,
        R => 0x52,
        S => 0x53,
        T => 0x54,
        U => 0x55,
        V => 0x56,
        W => 0x57,
        X => 0x58,
        Y => 0x59,
        Z => 0x5A,
        Num0 => 0x30,
        Num1 => 0x31,
        Num2 => 0x32,
        Num3 => 0x33,
        Num4 => 0x34,
        Num5 => 0x35,
        Num6 => 0x36,
        Num7 => 0x37,
        Num8 => 0x38,
        Num9 => 0x39,
        F1 => 0x70,
        F2 => 0x71,
        F3 => 0x72,
        F4 => 0x73,
        F5 => 0x74,
        F6 => 0x75,
        F7 => 0x76,
        F8 => 0x77,
        F9 => 0x78,
        F10 => 0x79,
        F11 => 0x7A,
        F12 => 0x7B,
        F13 => 0x7C,
        F14 => 0x7D,
        F15 => 0x7E,
        F16 => 0x7F,
        F17 => 0x80,
        F18 => 0x81,
        F19 => 0x82,
        F20 => 0x83,
        Space => 0x20,
        Insert => 0x2D,
        Delete => 0x2E,
        Home => 0x24,
        End => 0x23,
        PageUp => 0x21,
        PageDown => 0x22,
        ArrowLeft => 0x25,
        ArrowUp => 0x26,
        ArrowRight => 0x27,
        ArrowDown => 0x28,
        _ => return None,
    };
    Some(code)
}

fn vk_name(code: u16) -> String {
    match code {
        0x41..=0x5A => ((code as u8) as char).to_string(),
        0x30..=0x39 => ((code as u8) as char).to_string(),
        0x70..=0x87 => format!("F{}", code - 0x6F),
        0x20 => "Space".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x24 => "Home".into(),
        0x23 => "End".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0 => "None".into(),
        other => format!("0x{other:02X}"),
    }
}

fn vk_from_name(name: &str) -> Option<u16> {
    let upper = name.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix('F') {
        if let Ok(n) = rest.parse::<u16>() {
            if (1..=24).contains(&n) {
                return Some(0x6F + n);
            }
        }
    }
    if upper.len() == 1 {
        let ch = upper.as_bytes()[0];
        if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
            return Some(ch as u16);
        }
    }
    match upper.as_str() {
        "SPACE" => Some(0x20),
        "INSERT" => Some(0x2D),
        "DELETE" => Some(0x2E),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGEUP" => Some(0x21),
        "PAGEDOWN" => Some(0x22),
        "LEFT" => Some(0x25),
        "UP" => Some(0x26),
        "RIGHT" => Some(0x27),
        "DOWN" => Some(0x28),
        other if other.starts_with("0X") => u16::from_str_radix(&other[2..], 16).ok(),
        _ => None,
    }
}

pub fn default_launcher_hotkey() -> Hotkey {
    Hotkey {
        ctrl: true,
        alt: true,
        shift: false,
        win: false,
        key: 0x4D,
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/domain/hotkeys__tests.rs"]
mod tests;
