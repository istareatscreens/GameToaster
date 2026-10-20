use std::sync::{Mutex, MutexGuard};

use eframe::egui;

use crate::gui::common::components::{action, DialogContent, ModalDialog};
use crate::gui::common::theme;

pub(crate) const MAIN_WINDOW: egui::Vec2 = egui::vec2(1080.0, 800.0);

pub(crate) struct SettledModal {
    pub window: egui::Rect,

    pub bars: Vec<(egui::Rect, egui::Color32)>,
}

pub(crate) fn settled(
    context: &egui::Context,
    screen: Option<egui::Rect>,
    mut add: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let mut output = None;
    for frame in 0..12 {
        let input = egui::RawInput {
            screen_rect: screen,
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        output = Some(context.run_ui(input, &mut add));
    }
    output.expect("a frame was rendered")
}

pub(crate) fn settled_modal(id: egui::Id, mut show: impl FnMut(&egui::Context)) -> SettledModal {
    let context = egui::Context::default();
    theme::apply(&context);
    let main_window = egui::Rect::from_min_size(egui::Pos2::ZERO, MAIN_WINDOW);
    let output = settled(&context, Some(main_window), |ui| show(ui.ctx()));
    SettledModal {
        window: context
            .memory(|memory| memory.area_rect(id))
            .expect("the modal window was shown"),
        bars: painted_scroll_bars(&output, &context.global_style().spacing.scroll),
    }
}

pub(crate) fn scaffold_width() -> f32 {
    struct Fits;
    impl DialogContent for Fits {
        fn body(&mut self, ui: &mut egui::Ui) {
            ui.label("Body");
        }
        fn actions(&mut self, ui: &mut egui::Ui) {
            action(ui, "OK");
        }
    }
    let id = egui::Id::new("scaffold-width");
    settled_modal(id, |context| {
        ModalDialog::new("Title", id).show(context, &mut Fits);
    })
    .window
    .width()
}

pub(crate) fn painted_scroll_bars(
    output: &egui::FullOutput,
    scroll: &egui::style::ScrollStyle,
) -> Vec<(egui::Rect, egui::Color32)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if (shape.rect.width() - scroll.bar_width).abs() < 0.01
                    && shape.rect.height() >= scroll.handle_min_length =>
            {
                Some((shape.rect, shape.fill))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn painted_shapes(output: &egui::FullOutput) -> Vec<egui::Shape> {
    fn collect(shape: &egui::Shape, shapes: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(nested) => nested.iter().for_each(|shape| collect(shape, shapes)),
            shape => shapes.push(shape.clone()),
        }
    }
    let mut shapes = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut shapes);
    }
    shapes
}

pub(crate) fn painted_texts(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
    painted_shapes(output)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(text) => {
                Some((text.galley.text().to_owned(), text.visual_bounding_rect()))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn painted_on_both_surfaces(
    mut add: impl FnMut(&mut egui::Ui),
) -> [Vec<egui::Shape>; 2] {
    [false, true].map(|dialog| {
        let context = egui::Context::default();
        theme::apply(&context);
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            if dialog {
                theme::apply_widget_visuals(ui);
            }
            let surface = ui.visuals().clone();
            add(ui);
            assert!(
                *ui.visuals() == surface,
                "the surface's own visuals are restored"
            );
        });
        painted_shapes(&output)
    })
}

pub(crate) fn filled_rects(
    shapes: &[egui::Shape],
) -> Vec<(egui::Color32, egui::Stroke, egui::CornerRadius)> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill != egui::Color32::TRANSPARENT => {
                Some((rect.fill, rect.stroke, rect.corner_radius))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn painted_elisions(output: &egui::FullOutput) -> Vec<(String, bool)> {
    painted_shapes(output)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(text) => Some((text.galley.text().to_owned(), text.galley.elided)),
            _ => None,
        })
        .collect()
}

pub(crate) fn second_frame(
    hovered: bool,
    mut show: impl FnMut(&mut egui::Ui) -> egui::Response,
) -> (egui::FullOutput, egui::Response) {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut response = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| response = Some(show(ui)));
    let center = response.expect("the control was shown").rect.center();
    let input = egui::RawInput {
        events: if hovered {
            vec![egui::Event::PointerMoved(center)]
        } else {
            Vec::new()
        },
        ..Default::default()
    };
    let mut response = None;
    let output = context.run_ui(input, |ui| response = Some(show(ui)));
    (output, response.expect("the control was shown"))
}

pub(crate) fn click(position: egui::Pos2, screen: Option<egui::Rect>, time: f64) -> egui::RawInput {
    egui::RawInput {
        screen_rect: screen,
        time: Some(time),
        events: vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        ..Default::default()
    }
}

pub(crate) fn footer_actions(output: &egui::FullOutput, window: egui::Rect) -> Vec<egui::Rect> {
    let separator = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, .. }
                if points[0].y == points[1].y && window.contains(points[0]) =>
            {
                Some(points[0].y)
            }
            _ => None,
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        separator.is_finite(),
        "the dialog paints no footer separator"
    );
    let mut actions: Vec<egui::Rect> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.rect.top() > separator && window.contains_rect(shape.rect) =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .collect();
    actions.sort_by(|left, right| left.left().total_cmp(&right.left()));
    actions
}

pub(crate) fn close_cross(output: &egui::FullOutput) -> Option<egui::Pos2> {
    let diagonals: Vec<[egui::Pos2; 2]> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, .. }
                if points[0].x != points[1].x && points[0].y != points[1].y =>
            {
                Some(*points)
            }
            _ => None,
        })
        .collect();
    (diagonals.len() == 2).then(|| {
        let [a, b] = diagonals[0];
        a.lerp(b, 0.5)
    })
}

pub(crate) fn assert_pinned(
    output: &egui::FullOutput,
    window: egui::Rect,
    title: &str,
    error: &str,
) {
    let texts: Vec<(String, egui::Rect)> = painted_texts(output)
        .into_iter()
        .filter(|(_, rect)| window.contains(rect.center()))
        .collect();
    let find = |needle: &str| {
        texts
            .iter()
            .find(|(text, _)| text == needle)
            .map(|(_, rect)| *rect)
            .unwrap_or_else(|| panic!("{needle:?} is not painted in {texts:?}"))
    };
    let (title_rect, error_rect) = (find(title), find(error));
    assert!(
        title_rect.bottom() <= error_rect.top(),
        "{title_rect:?} {error_rect:?}"
    );
    for (text, rect) in &texts {
        if text != title && text != error {
            assert!(
                rect.top() >= error_rect.bottom() - 0.5,
                "{text:?} at {rect:?} is above the pinned error at {error_rect:?}"
            );
        }
    }
}

static LOCALE: Mutex<()> = Mutex::new(());

pub(crate) struct LocaleGuard {
    _lock: MutexGuard<'static, ()>,
}

impl Drop for LocaleGuard {
    fn drop(&mut self) {
        rust_i18n::set_locale("en");
    }
}

pub(crate) fn locale(code: &str) -> LocaleGuard {
    let lock = LOCALE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    rust_i18n::set_locale(code);
    LocaleGuard { _lock: lock }
}
