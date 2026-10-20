use std::hash::Hash;

use eframe::egui;

use super::measure;
use crate::gui::common::theme;

pub struct Combo<'a, T> {
    combo: egui::ComboBox,

    label: Option<String>,
    current: &'a mut T,
    choices: Vec<(T, String)>,
    placeholder: String,
    width: Width,
    resend_current: bool,
}

enum Width {
    Natural,

    Full,

    Fixed(f32),
}

impl<'a, T: PartialEq> Combo<'a, T> {
    pub fn new(id_salt: impl Hash, current: &'a mut T) -> Self {
        Self::over(egui::ComboBox::from_id_salt(id_salt), None, current)
    }

    pub fn labelled(label: impl Into<String>, current: &'a mut T) -> Self {
        let label = label.into();
        Self::over(
            egui::ComboBox::from_label(label.clone()),
            Some(label),
            current,
        )
    }

    fn over(combo: egui::ComboBox, label: Option<String>, current: &'a mut T) -> Self {
        Self {
            combo,
            label,
            current,
            choices: Vec::new(),
            placeholder: String::new(),
            width: Width::Natural,
            resend_current: false,
        }
    }

    pub fn choice(mut self, value: T, label: impl Into<String>) -> Self {
        self.choices.push((value, label.into()));
        self
    }

    pub fn choices<L: Into<String>>(mut self, choices: impl IntoIterator<Item = (T, L)>) -> Self {
        self.choices.extend(
            choices
                .into_iter()
                .map(|(value, label)| (value, label.into())),
        );
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn full_width(mut self) -> Self {
        self.width = Width::Full;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Width::Fixed(width);
        self
    }

    pub fn resend_current(mut self) -> Self {
        self.resend_current = true;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            combo,
            label,
            current,
            choices,
            placeholder,
            width,
            resend_current,
        } = self;
        let closed = choices
            .iter()
            .find(|(value, _)| *value == *current)
            .map_or(placeholder, |(_, label)| label.clone());

        let combo = combo.selected_text(closed).truncate();

        let room = match &label {
            None => ui.available_width(),
            Some(label) => {
                let label = measure(ui, |ui| ui.add(egui::Label::new(label.as_str()).extend()));
                (ui.available_width() - ui.spacing().item_spacing.x - label.x).max(0.0)
            }
        };

        let (combo, bound) = match width {
            Width::Natural => (combo, label.is_some().then_some(room)),
            Width::Full => (combo.width(room), label.is_some().then_some(room)),
            Width::Fixed(fixed) => (combo.width(fixed), Some(fixed)),
        };
        let mut changed = false;

        let enabled = ui.is_enabled();
        let show = |ui: &mut egui::Ui| {
            theme::with_widget_visuals(ui, |ui| {
                combo
                    .show_ui(ui, |ui| {
                        if !enabled {
                            ui.close();
                            return;
                        }
                        for (value, label) in choices {
                            let choice = ui.selectable_value(&mut *current, value, label);
                            changed |= choice.changed() || resend_current && choice.clicked();
                        }
                    })
                    .response
            })
        };
        let mut response = match bound {
            None => show(ui),
            Some(bound) => {
                let size = egui::vec2(bound, ui.spacing().interact_size.y);
                ui.allocate_ui(size, show).inner
            }
        };
        if changed {
            response.mark_changed();
        }
        response
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/combo__tests.rs"]
mod tests;
