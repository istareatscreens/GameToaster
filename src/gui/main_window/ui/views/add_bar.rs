use eframe::egui::{self, RichText, Vec2};
use egui_phosphor::regular as icons;
use rust_i18n::t;

use crate::gui::common::components::ButtonRole;
use crate::gui::common::theme;

use super::UiAction;

const ADD_BUTTON_HEIGHT: f32 = 42.0;

pub(super) struct AddBarResponses {
    add_game: egui::Response,
    plugins: egui::Response,
    hotkeys: egui::Response,
}

impl AddBarResponses {
    pub(super) fn discard(self) {
        drop((self.add_game, self.plugins, self.hotkeys));
    }
}

pub(super) fn add_game_bar(
    ui: &mut egui::Ui,
    busy: Option<&str>,
    actions: &mut Vec<UiAction>,
) -> AddBarResponses {
    let mut responses = None;
    ui.horizontal(|ui| {
        let add_game = ui.add_enabled(
            busy.is_none(),
            crate::gui::common::components::button(
                RichText::new(format!("{}  {}", icons::PLUS, t!("action.add_game"))),
                ButtonRole::Primary,
            )
            .min_size(Vec2::new(150.0, ADD_BUTTON_HEIGHT)),
        );
        if add_game.clicked() {
            actions.push(UiAction::AddGame);
        }
        ui.add_space(10.0);
        let plugins = ui.add_enabled(
            busy.is_none(),
            crate::gui::common::components::button(
                RichText::new(format!("{}  {}", icons::PLUG, t!("title.plugins")))
                    .size(theme::font_size::MEDIUM),
                ButtonRole::Primary,
            )
            .min_size(Vec2::new(132.0, ADD_BUTTON_HEIGHT)),
        );
        if plugins.clicked() {
            actions.push(UiAction::Plugins);
        }
        ui.add_space(10.0);
        let hotkeys = ui
            .add_enabled(
                busy.is_none(),
                crate::gui::common::components::button(
                    RichText::new(format!("{}  {}", icons::KEYBOARD, t!("action.hotkeys")))
                        .size(theme::font_size::MEDIUM),
                    ButtonRole::Primary,
                )
                .min_size(Vec2::new(132.0, ADD_BUTTON_HEIGHT)),
            )
            .on_hover_text(t!("tooltip.hotkeys_btn").to_string());
        if hotkeys.clicked() {
            actions.push(UiAction::OpenHotkeys);
        }
        if let Some(label) = busy {
            ui.add_space(12.0);
            ui.spinner();
            ui.label(crate::gui::common::components::text::status(label));
        }
        responses = Some(AddBarResponses {
            add_game,
            plugins,
            hotkeys,
        });
    });
    responses.expect("add-game bar must render its primary controls")
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/views/add_bar__tests.rs"]
mod tests;
