use eframe::egui;
use rust_i18n::t;

use super::{SettingsAction, SettingsDraft, TestStatus};
use crate::game_launcher_screen::controller::{ControllerDriverDescriptor, ControllerOptionKind};
use crate::game_launcher_screen::ControllerConfig;
use crate::gui::common::components::Combo;

pub(super) fn show(ui: &mut egui::Ui, draft: &mut SettingsDraft, action: &mut SettingsAction) {
    ui.heading(t!("controller.type.heading").to_string());
    let unknown_layout = draft.value.launcher_layout.clone();
    Combo::labelled(
        t!("controller.layout.select"),
        &mut draft.value.launcher_layout,
    )
    .choices(
        draft
            .layouts
            .iter()
            .map(|layout| (layout.id.to_string(), t!(layout.label_key))),
    )
    .placeholder(unknown_layout)
    .show(ui);
    let old_driver_id = draft.value.controller.driver_id.clone();
    Combo::labelled(
        t!("controller.type.select"),
        &mut draft.value.controller.driver_id,
    )
    .choices(
        draft
            .drivers
            .iter()
            .map(|driver| (driver.id.to_string(), t!(driver.label_key))),
    )
    .placeholder(old_driver_id.clone())
    .show(ui);
    if old_driver_id != draft.value.controller.driver_id {
        if let Some(driver) = draft
            .drivers
            .iter()
            .find(|driver| driver.id == draft.value.controller.driver_id)
        {
            draft.value.controller = driver.default_config();
            draft.value.launcher_layout = driver.recommended_layout.into();
        }
        draft.test_status = TestStatus::NotTested;
    }
    if let Some(driver) = draft
        .drivers
        .iter()
        .copied()
        .find(|driver| driver.id == draft.value.controller.driver_id)
    {
        show_driver_options(ui, driver, &mut draft.value.controller);
    }
    ui.horizontal(|ui| {
        if crate::gui::common::components::action(
            ui,
            if draft.testing {
                t!("controller.test.stop").to_string()
            } else {
                t!("controller.test.start").to_string()
            },
        )
        .clicked()
        {
            if draft.testing {
                *action = SettingsAction::StopTest;
            } else {
                match draft.controller_config() {
                    Ok(config) => *action = SettingsAction::Test(config),
                    Err(error) => {
                        draft.error = Some(t!("controller.error.test", detail => error).to_string())
                    }
                }
            }
        }
        if crate::gui::common::components::action(ui, t!("controller.test.auto_detect").to_string())
            .clicked()
        {
            *action = SettingsAction::AutoDetect;
        }
        ui.label(draft.test_status.resolve());
    });
    ui.label(t!("controller.test.live_buttons", buttons => draft.live_buttons.resolve()));
}

fn show_driver_options(
    ui: &mut egui::Ui,
    driver: &ControllerDriverDescriptor,
    config: &mut ControllerConfig,
) {
    for option in driver.options {
        let value = config
            .options
            .entry(option.key.into())
            .or_insert_with(|| option.default.into());
        match option.kind {
            ControllerOptionKind::HexU16 { .. } | ControllerOptionKind::Text { .. } => {
                ui.horizontal(|ui| {
                    ui.label(t!(option.label_key).to_string());
                    ui.text_edit_singleline(value);
                });
            }
            ControllerOptionKind::Boolean => {
                let mut checked = value == "true";
                if crate::gui::common::components::checkbox(
                    ui,
                    &mut checked,
                    t!(option.label_key).to_string(),
                )
                .changed()
                {
                    *value = checked.to_string();
                }
            }
            ControllerOptionKind::Choice(choices) => {
                let unknown = value.clone();
                Combo::labelled(t!(option.label_key), value)
                    .choices(
                        choices
                            .iter()
                            .map(|choice| (choice.value.to_string(), t!(choice.label_key))),
                    )
                    .placeholder(unknown)
                    .show(ui);
            }
        }
    }
}
