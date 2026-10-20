use eframe::egui::{self, Color32, Response, Sense, Stroke, Vec2, WidgetText};

use crate::gui::common::theme;

pub const SWITCH_SIZE: Vec2 = Vec2::new(54.0, 28.0);

pub fn checkbox(ui: &mut egui::Ui, checked: &mut bool, label: impl Into<WidgetText>) -> Response {
    theme::with_widget_visuals(ui, |ui| ui.add(egui::Checkbox::new(checked, label)))
}

pub fn radio<T: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    label: impl Into<WidgetText>,
) -> Response {
    theme::with_widget_visuals(ui, |ui| ui.radio_value(current, value, label))
}

pub fn switch(ui: &mut egui::Ui, on: &mut bool, enabled: bool) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, mut response) = ui.allocate_exact_size(SWITCH_SIZE, sense);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let fill = if !enabled {
        theme::DISABLED_TOGGLE
    } else if *on {
        theme::SELECTED_CYAN
    } else {
        theme::INACTIVE_LAVENDER
    };
    let pill = rect.height() / 2.0;
    let painter = ui.painter();
    painter.rect_filled(rect, pill, fill);
    painter.rect_stroke(rect, pill, theme::BORDER, egui::StrokeKind::Inside);
    let knob = egui::pos2(
        if *on {
            rect.right() - pill
        } else {
            rect.left() + pill
        },
        rect.center().y,
    );
    painter.circle(
        knob,
        pill - 3.0,
        Color32::WHITE,
        Stroke::new(1.0_f32, theme::INK),
    );
    response
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/toggle__tests.rs"]
mod tests;
