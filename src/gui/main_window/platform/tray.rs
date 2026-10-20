use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::egui;
use rust_i18n::t;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::launcher_gui::assets;
use crate::launcher_gui::model::{GameId, LauncherConfig};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    Restore,

    ToggleMainWindow,
    ToggleControllerScreen,
    Launch(GameId),
    Close,
}

enum RawTrayEvent {
    Action(TrayAction),
    Menu(MenuId),
}

pub struct TrayController {
    icon: TrayIcon,
    game_ids: HashMap<MenuId, GameId>,
    close_id: MenuId,
    restore_id: MenuId,
    controller_id: MenuId,

    queue: Arc<Mutex<Vec<RawTrayEvent>>>,
}

impl TrayController {
    pub fn new(
        launcher_dir: &std::path::Path,
        config: &LauncherConfig,
        ctx: egui::Context,
    ) -> Result<Self, String> {
        let queue: Arc<Mutex<Vec<RawTrayEvent>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let queue = queue.clone();
            let ctx = ctx.clone();
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::Click {
                    button,
                    button_state,
                    ..
                } = event
                {
                    if let Some(action) = click_action(button, button_state) {
                        if let Ok(mut queue) = queue.lock() {
                            queue.push(RawTrayEvent::Action(action));
                        }
                        ctx.request_repaint();
                    }
                }
            }));
        }
        {
            let queue = queue.clone();
            let ctx = ctx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if let Ok(mut queue) = queue.lock() {
                    queue.push(RawTrayEvent::Menu(event.id));
                }
                ctx.request_repaint();
            }));
        }
        let (rgba, width, height) = assets::icon_rgba(launcher_dir, 64)?;
        let icon = tray_icon::Icon::from_rgba(rgba, width, height)
            .map_err(|error| format!("failed to create tray icon: {error}"))?;
        let close_id = MenuId::new("launcher-close");
        let restore_id = MenuId::new("launcher-restore");
        let controller_id = MenuId::new("launcher-controller-screen");
        let (menu, game_ids) = build_menu(config, &restore_id, &controller_id, &close_id)?;
        let icon = TrayIconBuilder::new()
            .with_tooltip(t!("tray.tooltip"))
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .build()
            .map_err(|error| format!("failed to create tray icon: {error}"))?;
        Ok(Self {
            icon,
            game_ids,
            close_id,
            restore_id,
            controller_id,
            queue,
        })
    }

    pub fn rebuild(&mut self, config: &LauncherConfig) -> Result<(), String> {
        let (menu, game_ids) = build_menu(
            config,
            &self.restore_id,
            &self.controller_id,
            &self.close_id,
        )?;
        self.icon.set_menu(Some(Box::new(menu)));
        self.game_ids = game_ids;
        Ok(())
    }

    pub fn refresh_tooltip(&mut self) -> Result<(), String> {
        self.icon
            .set_tooltip(Some(t!("tray.tooltip")))
            .map_err(|error| format!("failed to set the tray tooltip: {error}"))
    }

    pub fn poll(&self) -> Vec<TrayAction> {
        let drained: Vec<RawTrayEvent> = match self.queue.lock() {
            Ok(mut queue) => std::mem::take(&mut *queue),
            Err(_) => Vec::new(),
        };
        let mut actions = Vec::new();
        for event in drained {
            match event {
                RawTrayEvent::Action(action) => actions.push(action),
                RawTrayEvent::Menu(id) => {
                    if let Some(action) = menu_action(
                        &id,
                        &self.restore_id,
                        &self.controller_id,
                        &self.close_id,
                        &self.game_ids,
                    ) {
                        actions.push(action);
                    }
                }
            }
        }
        actions
    }
}

fn click_action(button: MouseButton, state: MouseButtonState) -> Option<TrayAction> {
    (button == MouseButton::Left && state == MouseButtonState::Up).then_some(TrayAction::Restore)
}

fn menu_action(
    event: &MenuId,
    restore: &MenuId,
    controller: &MenuId,
    close: &MenuId,
    games: &HashMap<MenuId, GameId>,
) -> Option<TrayAction> {
    if event == restore {
        Some(TrayAction::ToggleMainWindow)
    } else if event == controller {
        Some(TrayAction::ToggleControllerScreen)
    } else if event == close {
        Some(TrayAction::Close)
    } else {
        games.get(event).copied().map(TrayAction::Launch)
    }
}

fn build_menu(
    config: &LauncherConfig,
    restore_id: &MenuId,
    controller_id: &MenuId,
    close_id: &MenuId,
) -> Result<(Menu, HashMap<MenuId, GameId>), String> {
    let menu = Menu::new();
    let mut ids = HashMap::new();
    for game in &config.games {
        let id = MenuId::new(format!("game-{}", game.id.0));
        let item = MenuItem::with_id(id.clone(), &game.label, true, None);
        menu.append(&item)
            .map_err(|error| format!("failed to add {} to tray menu: {error}", game.label))?;
        ids.insert(id, game.id);
    }
    if !config.games.is_empty() {
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|error| format!("failed to add tray separator: {error}"))?;
    }
    menu.append(&MenuItem::with_id(
        restore_id.clone(),
        t!("controller.tray.toggle_main").as_ref(),
        true,
        None,
    ))
    .map_err(|error| format!("failed to add main-window tray action: {error}"))?;
    menu.append(&MenuItem::with_id(
        controller_id.clone(),
        t!("controller.tray.toggle_screen").as_ref(),
        true,
        None,
    ))
    .map_err(|error| format!("failed to add controller-screen tray action: {error}"))?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|error| format!("failed to add tray separator: {error}"))?;
    let close = MenuItem::with_id(
        close_id.clone(),
        t!("controller.tray.close").as_ref(),
        true,
        None,
    );
    menu.append(&close)
        .map_err(|error| format!("failed to add Close to tray menu: {error}"))?;
    Ok((menu, ids))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/platform/tray__tests.rs"]
mod tests;
