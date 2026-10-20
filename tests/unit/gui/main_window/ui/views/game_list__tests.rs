use super::*;
use crate::gui::common::harness::{click, painted_shapes};
use crate::launcher_gui::model::CloseMode;

const SCREEN: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1080.0, 800.0));

const ROOM: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(40.0, 40.0), egui::pos2(1020.0, 200.0));
const GAME: GameId = GameId(3);

fn game() -> GameEntry {
    GameEntry {
        id: GAME,
        label: "Game".into(),
        controller_image: None,
        selected_target: std::path::PathBuf::from("C:/games/game.exe"),
        plugin: None,
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::default(),
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    }
}

fn frame_input(frame: u32) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(SCREEN),
        time: Some(f64::from(frame) * 0.1),
        ..Default::default()
    }
}

fn row_frame(
    context: &egui::Context,
    input: egui::RawInput,
    is_default: bool,
    disabled: bool,
    actions: &mut Vec<UiAction>,
) -> egui::FullOutput {
    context.run_ui(input, |root| {
        root.scope_builder(egui::UiBuilder::new().max_rect(ROOM), |ui| {
            game_row(ui, &game(), None, is_default, disabled, actions);
        });
    })
}

fn laid_out(
    context: &egui::Context,
    is_default: bool,
    disabled: bool,
    actions: &mut Vec<UiAction>,
) -> (egui::FullOutput, egui::Rect) {
    let mut output = None;
    for frame in 0..2 {
        output = Some(row_frame(
            context,
            frame_input(frame),
            is_default,
            disabled,
            actions,
        ));
    }
    let output = output.unwrap();

    let mut switches: Vec<egui::Rect> = painted_shapes(&output)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.rect.size() == components::SWITCH_SIZE => {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect();
    switches.dedup();
    assert_eq!(switches.len(), 1, "one default switch: {switches:?}");
    (output, switches[0])
}

#[test]
fn the_default_switch_sets_and_clears_the_default_game() {
    for (is_default, expected) in [(false, Some(GAME)), (true, None)] {
        let context = egui::Context::default();
        theme::apply(&context);
        let mut actions = Vec::new();
        let (_, switch) = laid_out(&context, is_default, false, &mut actions);
        assert!(actions.is_empty(), "{actions:?}");
        let _ = row_frame(
            &context,
            click(switch.center(), Some(SCREEN), 0.3),
            is_default,
            false,
            &mut actions,
        );
        assert_eq!(actions, [UiAction::SetDefault(expected)], "{is_default}");
    }
}

#[test]
fn a_disabled_row_sends_nothing() {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut actions = Vec::new();
    let (_, switch) = laid_out(&context, false, true, &mut actions);
    let _ = row_frame(
        &context,
        click(switch.center(), Some(SCREEN), 0.3),
        false,
        true,
        &mut actions,
    );
    assert!(actions.is_empty(), "{actions:?}");
}

#[test]
fn the_rows_controls_end_at_its_inner_right_edge() {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut actions = Vec::new();
    let (output, switch) = laid_out(&context, false, false, &mut actions);
    let shapes = painted_shapes(&output);
    let frame = shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == theme::ROW_CREAM => Some(rect.rect),
            _ => None,
        })
        .max_by(|a, b| a.area().total_cmp(&b.area()))
        .expect("the row's frame is painted");

    let mut tiles: Vec<egui::Rect> = shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.rect.size() == components::TILE_SIZE => Some(rect.rect),
            _ => None,
        })
        .collect();
    tiles.dedup();
    assert_eq!(tiles.len(), 6, "{tiles:?}");

    let row = theme::frames::list_row();
    let inner_right = frame.right() - row.inner_margin.rightf() - row.stroke.width;
    let last = tiles
        .iter()
        .map(|tile| tile.right())
        .fold(f32::MIN, f32::max);
    assert!(
        (last - inner_right).abs() < 0.5,
        "the last tile ends at {last}, the row's inner edge is at {inner_right}"
    );
    assert!(
        switch.right()
            < tiles
                .iter()
                .map(|tile| tile.left())
                .fold(f32::MAX, f32::min),
        "the switch comes before the tiles"
    );
}

#[test]
fn a_launch_that_started_the_game_reads_running_instead_of_its_last_progress() {
    use crate::plugin::runtime::channel::Progress;
    use crate::plugin::runtime::{Message, RuntimeStatus, Stage};

    let _locale = crate::gui::common::harness::locale("en");
    let mut status = RuntimeStatus::default();
    status.apply(&Message::Progress(Progress {
        stage: Stage::Launching,
        percent: Some(90.0),
        text: "Launching game".into(),
    }));
    status.apply(&Message::Progress(Progress {
        stage: Stage::Running,
        percent: None,
        text: "Game running".into(),
    }));
    assert_eq!(
        runtime_status_text(&status).unwrap(),
        "Running • Game running"
    );
}

#[test]
fn the_status_line_names_the_stage_with_its_percent_and_shows_unknown_stages_verbatim() {
    use crate::plugin::runtime::channel::Progress;
    use crate::plugin::runtime::{GameMode, GameState, Message, RuntimeStatus, Stage};

    let _locale = crate::gui::common::harness::locale("en");
    let mut status = RuntimeStatus::default();
    status.apply(&Message::Progress(Progress {
        stage: Stage::Patching,
        percent: Some(40.0),
        text: String::new(),
    }));
    assert_eq!(
        runtime_status_text(&status).unwrap(),
        format!("{} — 40%", t!("plugin_runtime.stage.patching"))
    );

    status.apply(&Message::Progress(Progress {
        stage: Stage::Other("defragging".into()),
        percent: None,
        text: "almost there".into(),
    }));
    assert_eq!(
        runtime_status_text(&status).unwrap(),
        "defragging • almost there"
    );

    status.apply(&Message::GameState(GameState {
        credits: Some(2),
        paid_play: Some(true),
        free_play: false,
        mode: GameMode::ActivePlay,
    }));
    let text = runtime_status_text(&status).unwrap();
    assert!(text.starts_with("defragging • almost there • "), "{text}");
    assert!(
        text.contains(&t!("plugin_runtime.paid_play").to_string()),
        "{text}"
    );
    assert!(!text.contains("%{"), "a placeholder leaked: {text}");
}
