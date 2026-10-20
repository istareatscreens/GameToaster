use eframe::egui::{self, RichText};
use rust_i18n::t;

use crate::gui::common::components::{self, text, Combo};
use crate::gui::common::theme;
use crate::launcher_gui::model::LauncherConfig;

use super::{SettingKey, UiAction};

const ROW_HEIGHT: f32 = 22.0;

const COMBO_WIDTH: f32 = 310.0;

const ROW_SPACING: f32 = 6.0;

const SETTINGS_BLOCK_PADDING: f32 = 12.0;

const CONTROLLER_CARD_HEIGHT: f32 = 112.0;

const SHOW_BLOCK_EXIT_ON_CREDIT: bool = false;

struct SettingRowSpec {
    key: SettingKey,
    label: fn() -> String,
    current: fn(&LauncherConfig) -> bool,
    shown: bool,
}

fn label_auto_hide() -> String {
    t!("settings.auto_hide").to_string()
}
fn label_hide_cmd() -> String {
    t!("settings.hide_cmd").to_string()
}
fn label_launch_default() -> String {
    t!("settings.launch_default").to_string()
}
fn label_pause_failure() -> String {
    t!("settings.pause_failure").to_string()
}
fn label_block_exit_on_credit() -> String {
    t!("settings.block_exit_on_credit").to_string()
}
fn current_auto_hide(config: &LauncherConfig) -> bool {
    config.settings.auto_hide_on_launch
}
fn current_hide_cmd(config: &LauncherConfig) -> bool {
    !config.settings.show_command_windows
}
fn current_launch_default(config: &LauncherConfig) -> bool {
    config.settings.launch_default_on_open
}
fn current_pause_failure(config: &LauncherConfig) -> bool {
    config.settings.pause_on_failure
}
fn current_block_exit_on_credit(config: &LauncherConfig) -> bool {
    config.settings.block_exit_on_credit
}

const LEFT_SETTING_ROWS: &[SettingRowSpec] = &[
    SettingRowSpec {
        key: SettingKey::AutoHide,
        label: label_auto_hide,
        current: current_auto_hide,
        shown: true,
    },
    SettingRowSpec {
        key: SettingKey::HideCommandWindows,
        label: label_hide_cmd,
        current: current_hide_cmd,
        shown: true,
    },
    SettingRowSpec {
        key: SettingKey::LaunchDefaultOnOpen,
        label: label_launch_default,
        current: current_launch_default,
        shown: true,
    },
    SettingRowSpec {
        key: SettingKey::PauseOnFailure,
        label: label_pause_failure,
        current: current_pause_failure,
        shown: true,
    },
    SettingRowSpec {
        key: SettingKey::BlockExitOnCredit,
        label: label_block_exit_on_credit,
        current: current_block_exit_on_credit,
        shown: SHOW_BLOCK_EXIT_ON_CREDIT,
    },
];

fn config_rows() -> impl Iterator<Item = &'static SettingRowSpec> {
    LEFT_SETTING_ROWS.iter().filter(|row| row.shown)
}

pub(super) fn left_row_count() -> usize {
    config_rows().count() + 1
}

#[cfg(test)]
pub(super) fn draws_setting(key: SettingKey) -> bool {
    config_rows().any(|row| row.key == key)
}

pub(super) fn reserved_height(ui: &mut egui::Ui) -> f32 {
    let rows = left_row_count() as f32;
    let left = rows * ROW_HEIGHT + (rows - 1.0) * ROW_SPACING;

    let combo = components::measure(ui, |ui| {
        Combo::new("settings-reserve-combo", &mut ())
            .placeholder("M")
            .width(COMBO_WIDTH)
            .show(ui)
    })
    .y;
    let right = 2.0 * (ROW_HEIGHT + combo) + 3.0 * ROW_SPACING;
    left.max(right).max(CONTROLLER_CARD_HEIGHT) + SETTINGS_BLOCK_PADDING
}

pub(super) fn settings(
    ui: &mut egui::Ui,
    config: &LauncherConfig,
    disabled: bool,
    controller_open: bool,
    controller_status: &str,
    actions: &mut Vec<UiAction>,
) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 26.0;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = ROW_SPACING;
            for row in config_rows() {
                setting_checkbox(
                    ui,
                    &(row.label)(),
                    (row.current)(config),
                    disabled,
                    row.key,
                    actions,
                );
            }

            let startup_on = crate::launcher_gui::platform::startup::is_enabled();
            if let Some(on) =
                checkbox_row(ui, &t!("settings.launch_on_startup"), startup_on, disabled)
            {
                actions.push(UiAction::SetLaunchOnStartup(on));
            }
        });
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = ROW_SPACING;
            language_combo(ui, config, disabled, actions);
            let monitors = crate::launcher_gui::monitors::enumerate();
            screen_combo(ui, config, &monitors, disabled, actions);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            controller_controls(
                ui,
                disabled,
                config.settings.controller_screen.enabled,
                controller_open,
                controller_status,
                actions,
            );
        });
    });
}

fn controller_controls(
    ui: &mut egui::Ui,
    disabled: bool,
    controller_enabled: bool,
    controller_open: bool,
    controller_status: &str,
    actions: &mut Vec<UiAction>,
) {
    ui.vertical(|ui| {
        ui.set_width(238.0);
        ui.spacing_mut().item_spacing.y = ROW_SPACING;
        ui.label(
            RichText::new(t!("controller.card.heading").to_string())
                .size(theme::font_size::TINY)
                .strong()
                .color(theme::MUTED),
        );

        let enable_label = t!("controller.card.enable");
        setting_checkbox(
            ui,
            enable_label.as_ref(),
            controller_enabled,
            disabled,
            SettingKey::ControllerEnabled,
            actions,
        );
        ui.label(
            RichText::new(controller_status)
                .size(theme::font_size::TINY)
                .color(if controller_open {
                    theme::INK
                } else {
                    theme::MUTED
                }),
        );
        ui.horizontal(|ui| {
            let screen = if controller_open {
                t!("controller.card.close")
            } else {
                t!("controller.card.open")
            };
            if components::action_enabled(ui, !disabled && controller_enabled, screen).clicked() {
                actions.push(UiAction::ToggleControllerScreen);
            }
            if components::action_enabled(ui, !disabled, t!("controller.card.settings")).clicked() {
                actions.push(UiAction::OpenControllerSettings);
            }
        });
    });
}

fn language_combo(
    ui: &mut egui::Ui,
    config: &LauncherConfig,
    disabled: bool,
    actions: &mut Vec<UiAction>,
) {
    let mut code = crate::launcher_gui::i18n::resolved_code(&config.settings.language);

    let combo = Combo::new("launcher-language", &mut code)
        .choices(
            crate::launcher_gui::i18n::LOCALES
                .iter()
                .map(|locale| (locale.code, locale.native_name)),
        )
        .placeholder("English")
        .width(COMBO_WIDTH)
        .resend_current();
    if captioned(ui, t!("settings.language"), disabled, |ui| combo.show(ui)).changed() {
        actions.push(UiAction::SetLanguage(code));
    }
}

fn screen_combo(
    ui: &mut egui::Ui,
    config: &LauncherConfig,
    monitors: &[crate::launcher_gui::monitors::MonitorInfo],
    disabled: bool,
    actions: &mut Vec<UiAction>,
) {
    let main_display = t!("settings.main_display").to_string();
    let monitor_choices = monitors.iter().map(|monitor| {
        let (number, width, height) = (monitor.index + 1, monitor.width, monitor.height);
        let label = if monitor.primary {
            t!("settings.monitor_label_primary", number => number, width => width, height => height)
        } else {
            t!("settings.monitor_label", number => number, width => width, height => height)
        };
        (Some(monitor.index), label.to_string())
    });

    let mut adapter = config.settings.adapter;
    let combo = Combo::new("launcher-adapter", &mut adapter)
        .choices(std::iter::once((None, main_display.clone())).chain(monitor_choices))
        .placeholder(main_display)
        .width(COMBO_WIDTH)
        .resend_current();
    if captioned(ui, t!("settings.screen"), disabled, |ui| combo.show(ui)).changed() {
        actions.push(UiAction::SetAdapter(adapter));
    }
}

fn captioned(
    ui: &mut egui::Ui,
    caption: impl Into<String>,
    disabled: bool,
    control: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    row(ui, |ui| ui.label(text::hint(caption)));
    ui.add_enabled_ui(!disabled, control).inner
}

fn setting_checkbox(
    ui: &mut egui::Ui,
    label: &str,
    current: bool,
    disabled: bool,
    key: SettingKey,
    actions: &mut Vec<UiAction>,
) {
    if let Some(value) = checkbox_row(ui, label, current, disabled) {
        actions.push(UiAction::SetSetting(key, value));
    }
}

fn checkbox_row(ui: &mut egui::Ui, label: &str, current: bool, disabled: bool) -> Option<bool> {
    let mut value = current;
    let response = row(ui, |ui| {
        ui.add_enabled_ui(!disabled, |ui| {
            components::checkbox(
                ui,
                &mut value,
                RichText::new(label).size(theme::font_size::COMPACT),
            )
        })
        .inner
    });
    response.changed().then_some(value)
}

fn row<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        add,
    )
    .inner
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/views/settings__tests.rs"]
mod tests;
