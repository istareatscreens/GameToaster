use eframe::egui::{self, Response, Stroke, Vec2, Widget, WidgetText};

use crate::gui::common::theme;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonRole {
    Primary,
    Secondary,
    Quiet,
    Destructive,
    Selected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RoleStyle {
    idle: egui::Color32,
    hovered: egui::Color32,
    active: egui::Color32,
    disabled: egui::Color32,
}

fn role_style(role: ButtonRole) -> RoleStyle {
    let (idle, hovered, active) = match role {
        ButtonRole::Primary => (
            theme::PRIMARY_ORANGE,
            theme::PRIMARY_ORANGE.gamma_multiply(1.08),
            theme::SELECTED_CYAN,
        ),
        ButtonRole::Secondary => (
            theme::ROW_CREAM,
            theme::BANNER_LAVENDER,
            theme::SELECTED_CYAN,
        ),
        ButtonRole::Quiet => (
            theme::ICON_TILE,
            theme::BANNER_LAVENDER,
            theme::INACTIVE_LAVENDER,
        ),
        ButtonRole::Destructive => (theme::SALMON, theme::CLOSE_HOVER, theme::ERROR),
        ButtonRole::Selected => (
            theme::SELECTED_CYAN,
            theme::SELECTED_CYAN.gamma_multiply(1.08),
            theme::PRIMARY_ORANGE,
        ),
    };
    RoleStyle {
        idle,
        hovered,
        active,
        disabled: idle.gamma_multiply(0.62),
    }
}

pub struct SemanticButton<'a> {
    content: WidgetText,
    role: ButtonRole,
    borderless: bool,
    selected: bool,
    minimum: Vec2,
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl SemanticButton<'_> {
    pub fn min_size(mut self, minimum: Vec2) -> Self {
        self.minimum = minimum;
        self
    }
}

impl Widget for SemanticButton<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let style = role_style(self.role);
        let stroke = if self.borderless {
            Stroke::NONE
        } else {
            theme::BORDER
        };

        let saved = ui.visuals().widgets.clone();
        {
            let widgets = &mut ui.visuals_mut().widgets;
            for (visual, fill) in [
                (&mut widgets.noninteractive, style.disabled),
                (&mut widgets.inactive, style.idle),
                (&mut widgets.hovered, style.hovered),
                (&mut widgets.active, style.active),
                (&mut widgets.open, style.hovered),
            ] {
                visual.bg_fill = fill;
                visual.weak_bg_fill = fill;
                visual.bg_stroke = stroke;
            }
        }
        let response = ui.add(
            egui::Button::new(self.content)
                .selected(self.selected)
                .min_size(self.minimum)
                .stroke(stroke)
                .corner_radius(if self.borderless { 4.0 } else { 6.0 })
                .wrap_mode(egui::TextWrapMode::Extend),
        );
        ui.visuals_mut().widgets = saved;
        response
    }
}

fn semantic_button(
    content: impl Into<WidgetText>,
    role: ButtonRole,
    borderless: bool,
    selected: bool,
    minimum: Vec2,
) -> SemanticButton<'static> {
    SemanticButton {
        content: content.into(),
        role,
        borderless,
        selected,
        minimum,
        lifetime: std::marker::PhantomData,
    }
}

pub fn button(content: impl Into<WidgetText>, role: ButtonRole) -> SemanticButton<'static> {
    semantic_button(
        content,
        role,
        false,
        role == ButtonRole::Selected,
        Vec2::ZERO,
    )
}

pub fn borderless_button(
    content: impl Into<WidgetText>,
    role: ButtonRole,
) -> SemanticButton<'static> {
    semantic_button(
        content,
        role,
        true,
        role == ButtonRole::Selected,
        Vec2::ZERO,
    )
}

pub fn choice_button(
    content: impl Into<WidgetText>,
    selected: bool,
    minimum: Vec2,
) -> SemanticButton<'static> {
    semantic_button(
        content,
        if selected {
            ButtonRole::Selected
        } else {
            ButtonRole::Secondary
        },
        false,
        selected,
        minimum,
    )
}

pub fn action(ui: &mut egui::Ui, content: impl Into<WidgetText>) -> Response {
    ui.add(button(content, ButtonRole::Primary))
}

pub fn action_enabled(
    ui: &mut egui::Ui,
    enabled: bool,
    content: impl Into<WidgetText>,
) -> Response {
    ui.add_enabled(enabled, button(content, ButtonRole::Primary))
}

pub fn action_sized(
    ui: &mut egui::Ui,
    size: impl Into<Vec2>,
    content: impl Into<WidgetText>,
) -> Response {
    ui.add_sized(size, button(content, ButtonRole::Primary))
}

pub fn copy_button(ui: &mut egui::Ui, label: impl Into<WidgetText>, text: &str) -> Response {
    let response = ui.add(button(label, ButtonRole::Secondary));
    if response.clicked() {
        ui.ctx().copy_text(text.to_string());
    }
    response
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/button__tests.rs"]
mod tests;
