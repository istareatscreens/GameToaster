use super::*;
use crate::plugin::runtime::channel::messages::{
    GameMode, LogLevel, LogMessage, ResultMessage, ResultStatus, Stage,
};

#[test]
fn the_last_progress_and_game_state_are_kept_and_results_and_logs_carry_no_status() {
    let mut status = RuntimeStatus::default();
    assert_eq!(status, RuntimeStatus::default());
    status.apply(&Message::Log(LogMessage {
        level: LogLevel::Info,
        text: "hello".into(),
    }));
    assert_eq!(status, RuntimeStatus::default());
    status.apply(&Message::Progress(Progress {
        stage: Stage::Patching,
        percent: Some(10.0),
        text: String::new(),
    }));
    status.apply(&Message::Progress(Progress {
        stage: Stage::Verifying,
        percent: Some(75.0),
        text: "checking".into(),
    }));
    status.apply(&Message::GameState(GameState {
        credits: Some(1),
        paid_play: Some(false),
        free_play: false,
        mode: GameMode::Attract,
    }));
    let before = status.clone();
    status.apply(&Message::Result(ResultMessage {
        status: ResultStatus::Ok,
        text: "done".into(),
    }));
    assert_eq!(status, before);
    assert_eq!(status.progress.as_ref().unwrap().stage, Stage::Verifying);
    assert_eq!(status.progress.as_ref().unwrap().text, "checking");
    assert_eq!(status.game_state.unwrap().credits, Some(1));
    assert_ne!(status, RuntimeStatus::default());
}
