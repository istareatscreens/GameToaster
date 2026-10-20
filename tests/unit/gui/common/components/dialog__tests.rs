use super::*;
use crate::gui::common::components::action;
use crate::gui::common::harness::{
    close_cross, painted_scroll_bars, painted_texts, settled, MAIN_WINDOW,
};

const ID: &str = "dialog-under-test";

struct Probe {
    lines: usize,
    error: Option<String>,
    action: Option<egui::Rect>,
}

impl Probe {
    fn new(lines: usize, error_lines: usize) -> Self {
        Self {
            lines,
            error: (error_lines > 0).then(|| numbered("error line", error_lines)),
            action: None,
        }
    }
}

impl DialogContent for Probe {
    fn body(&mut self, ui: &mut egui::Ui) {
        for line in 0..self.lines {
            ui.label(format!("body line {line}"));
        }
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        self.action = Some(action(ui, "Done").rect);
    }
}

fn numbered(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|line| format!("{prefix} {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn main_window() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, MAIN_WINDOW)
}

struct Shown {
    window: egui::Rect,
    output: egui::FullOutput,
    bars: Vec<(egui::Rect, egui::Rect)>,
}

impl Shown {
    fn text(&self, needle: &str) -> egui::Rect {
        painted_texts(&self.output)
            .into_iter()
            .find(|(text, _)| text.starts_with(needle))
            .map(|(_, rect)| rect)
            .unwrap_or_else(|| panic!("{needle:?} was not painted"))
    }
}

fn shown(probe: &mut Probe) -> Shown {
    let context = egui::Context::default();
    theme::apply(&context);
    let output = settled(&context, Some(main_window()), |ui| {
        ModalDialog::new("Probe", egui::Id::new(ID)).show(ui.ctx(), probe);
    });
    let painted = painted_scroll_bars(&output, &context.global_style().spacing.scroll);
    assert!(painted.len().is_multiple_of(2), "{painted:?}");
    Shown {
        window: context
            .memory(|memory| memory.area_rect(egui::Id::new(ID)))
            .expect("the dialog was shown"),
        bars: painted
            .chunks(2)
            .map(|pair| (pair[0].0, pair[1].0))
            .collect(),
        output,
    }
}

#[test]
fn a_tall_body_scrolls_in_a_624_point_track_above_the_actions() {
    let mut probe = Probe::new(80, 0);
    let shown = shown(&mut probe);
    assert_eq!(shown.bars.len(), 1, "{:?}", shown.bars);
    let track = shown.bars[0].0;
    assert!((track.height() - BODY_MAX_HEIGHT).abs() < 0.01, "{track:?}");
    let action = probe.action.unwrap();
    assert!(action.top() > track.bottom(), "{action:?} {track:?}");
    assert!(shown.window.contains_rect(action), "{action:?}");
    assert!(
        main_window().contains_rect(shown.window),
        "{:?}",
        shown.window
    );
}

#[test]
fn a_short_body_shows_no_scroll_bar_and_a_separator_above_the_actions() {
    let mut probe = Probe::new(3, 0);
    let shown = shown(&mut probe);
    assert!(shown.bars.is_empty(), "{:?}", shown.bars);
    let last_line = shown.text("body line 2");
    let action = probe.action.unwrap();
    let separator = shown.output.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::LineSegment { points, .. }
            if points[0].y == points[1].y
                && points[0].y > last_line.bottom()
                && points[0].y < action.top())
    });
    assert!(separator, "no separator between the body and the actions");
    assert!(
        main_window().contains_rect(shown.window),
        "{:?}",
        shown.window
    );
}

#[test]
fn an_error_is_pinned_under_the_title_and_the_body_gives_up_its_height() {
    let without = shown(&mut Probe::new(80, 0));
    let with = shown(&mut Probe::new(80, 2));
    assert_eq!(with.bars.len(), 1, "{:?}", with.bars);
    let (track_without, track_with) = (without.bars[0].0, with.bars[0].0);

    let title = with.text("Probe");
    let error = with.text("error line 0");
    assert!(title.bottom() <= error.top(), "{title:?} {error:?}");
    assert!(
        error.bottom() <= track_with.top(),
        "{error:?} {track_with:?}"
    );

    assert!(track_with.height() < BODY_MAX_HEIGHT, "{track_with:?}");
    assert!(
        (track_with.bottom() - track_without.bottom()).abs() < 0.5,
        "the body keeps its bottom edge: {track_with:?} {track_without:?}"
    );
    assert!(
        (with.window.height() - without.window.height()).abs() < 0.5,
        "an error never makes the dialog taller: {:?} {:?}",
        with.window,
        without.window
    );
    assert!(
        main_window().contains_rect(with.window),
        "{:?}",
        with.window
    );
}

#[test]
fn an_error_longer_than_its_limit_scrolls_in_its_own_156_point_area() {
    let mut probe = Probe::new(3, 40);
    let shown = shown(&mut probe);
    assert_eq!(
        shown.bars.len(),
        1,
        "only the error scrolls: {:?}",
        shown.bars
    );
    let track = shown.bars[0].0;
    assert!(
        (track.height() - ERROR_MAX_HEIGHT).abs() < 0.01,
        "{track:?}"
    );
    assert!(
        track.bottom() < shown.text("body line 0").top(),
        "{track:?}"
    );
    assert!(
        main_window().contains_rect(shown.window),
        "{:?}",
        shown.window
    );
}

#[test]
fn a_dialog_is_as_wide_as_its_body_and_its_scroll_bar_whether_it_scrolls_or_not() {
    let spacing = &theme_spacing();
    let expected =
        BODY_WIDTH + spacing.scroll.allocated_width() + frames::modal().total_margin().sum().x;
    for lines in [3, 80] {
        let shown = shown(&mut Probe::new(lines, 0));
        assert_eq!(shown.bars.is_empty(), lines == 3, "{:?}", shown.bars);
        assert!(
            (shown.window.width() - expected).abs() < 0.5,
            "{lines} lines: {} against {expected}",
            shown.window.width()
        );
    }
}

fn theme_spacing() -> egui::style::Spacing {
    let context = egui::Context::default();
    theme::apply(&context);
    context.global_style().spacing.clone()
}

#[test]
fn a_dialog_has_no_title_bar_close_button() {
    let mut probe = Probe::new(3, 0);
    assert!(close_cross(&shown(&mut probe).output).is_none());
}
