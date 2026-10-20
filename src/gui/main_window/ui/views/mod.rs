use eframe::egui::{self, RichText, Sense, Vec2};
use rust_i18n::t;

use crate::gui::common::components;
use crate::gui::common::theme;
use crate::launcher_gui::model::{GameId, LauncherConfig};
use crate::plugin::runtime::RuntimeStatus;
use std::collections::BTreeMap;

mod add_bar;
mod game_list;
mod settings;

use add_bar::add_game_bar;
use game_list::game_list;
use settings::settings;

const TITLE_BAR_HEIGHT: f32 = 38.0;
const CLOSE_BUTTON_WIDTH: f32 = 46.0;
const MINIMIZE_HEIGHT: f32 = 48.0;
const MINIMIZE_PANEL_HEIGHT: f32 = 72.0;

const CONTENT_MARGIN: egui::Margin = egui::Margin {
    left: 22,
    right: 22,
    top: 16,
    bottom: 16,
};

const MINIMIZE_PANEL_MARGIN: egui::Margin = egui::Margin {
    left: 22,
    right: 22,
    top: 8,
    bottom: 16,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingKey {
    AutoHide,
    HideCommandWindows,
    LaunchDefaultOnOpen,
    PauseOnFailure,

    BlockExitOnCredit,

    ControllerEnabled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiAction {
    None,
    AddGame,
    Plugins,
    OpenHotkeys,
    Launch(GameId),
    SetDefault(Option<GameId>),
    Rename(GameId),
    Config(GameId),
    OpenFolder(GameId),
    CloseGame(GameId),
    ConfigureClose(GameId),
    Remove(GameId),
    SetSetting(SettingKey, bool),

    SetLaunchOnStartup(bool),
    SetAdapter(Option<u32>),
    SetLanguage(&'static str),
    OpenControllerSettings,
    ToggleControllerScreen,
    MinimizeToTray,
    RequestClose,
}

pub fn title_bar(root: &mut egui::Ui) -> UiAction {
    let context = root.ctx().clone();
    let mut action = UiAction::None;
    egui::Panel::top("launcher-title-bar")
        .exact_size(TITLE_BAR_HEIGHT)
        .frame(theme::frames::title_bar())
        .show_inside(root, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.horizontal(|ui| {
                let drag_width = (ui.available_width() - CLOSE_BUTTON_WIDTH).max(20.0);
                let drag = ui.allocate_response(
                    Vec2::new(drag_width, TITLE_BAR_HEIGHT),
                    Sense::click_and_drag(),
                );
                if drag.drag_started() || drag.dragged() {
                    context.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if components::window_close_button(
                    ui,
                    Vec2::new(CLOSE_BUTTON_WIDTH, TITLE_BAR_HEIGHT),
                )
                .on_hover_text(t!("tooltip.close_window").to_string())
                .clicked()
                {
                    action = UiAction::RequestClose;
                }
            });
        });
    action
}

pub fn main_content(
    root: &mut egui::Ui,
    config: &LauncherConfig,
    plugin_status: &BTreeMap<GameId, RuntimeStatus>,
    busy: Option<&str>,
    controller_open: bool,
    controller_status: &str,
) -> Vec<UiAction> {
    let mut actions = Vec::new();
    minimize_panel(root, &mut actions);
    egui::CentralPanel::default()
        .frame(theme::frames::canvas(CONTENT_MARGIN))
        .show_inside(root, |ui| {
            theme::paint_vertical_gradient(
                ui.painter(),
                ui.max_rect().expand2(egui::vec2(
                    f32::from(CONTENT_MARGIN.left),
                    f32::from(CONTENT_MARGIN.top),
                )),
                theme::SKY_TOP,
                theme::SKY_BOTTOM,
            );
            add_game_bar(ui, busy, &mut actions).discard();
            ui.add_space(14.0);

            let settings_height = settings::reserved_height(ui);
            let reserved = settings_height + 14.0;
            let list_height = (ui.available_height() - reserved).max(120.0);

            game_list(
                ui,
                config,
                plugin_status,
                busy.is_some(),
                list_height,
                &mut actions,
            );

            ui.add_space(14.0);
            settings(
                ui,
                config,
                busy.is_some(),
                controller_open,
                controller_status,
                &mut actions,
            );
        });
    actions
}

fn minimize_panel(root: &mut egui::Ui, actions: &mut Vec<UiAction>) -> egui::Response {
    let mut response = None;
    egui::Panel::bottom("launcher-minimize-panel")
        .exact_size(MINIMIZE_PANEL_HEIGHT)
        .frame(theme::frames::canvas(MINIMIZE_PANEL_MARGIN))
        .show_inside(root, |ui| {
            let rendered = components::action_sized(
                ui,
                [ui.available_width(), MINIMIZE_HEIGHT],
                RichText::new(t!("action.minimize_tray").to_string()).size(theme::font_size::LARGE),
            );
            if rendered.clicked() {
                actions.push(UiAction::MinimizeToTray);
            }
            response = Some(rendered);
        });
    response.expect("minimize panel must render its action")
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/views/mod__tests.rs"]
mod tests;
