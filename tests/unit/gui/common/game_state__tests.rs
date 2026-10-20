use super::*;

fn state(credits: u32, mode: GameMode) -> GameState {
    GameState {
        credits: Some(credits),
        paid_play: Some(mode == GameMode::ActivePlay),
        free_play: false,
        mode,
    }
}

#[test]
fn every_unknown_or_disabled_state_fails_open() {
    let hub = GameStateHub::default();
    assert!(!should_block_game_exit(true, hub.read()));
    hub.apply(&state(3, GameMode::ActivePlay));
    assert!(!should_block_game_exit(false, hub.read()));
    hub.clear();
    assert!(!should_block_game_exit(true, hub.read()));
}

#[test]
fn unknown_payment_and_free_play_fail_open() {
    let hub = GameStateHub::default();
    hub.apply(&GameState {
        credits: None,
        paid_play: None,
        free_play: false,
        mode: GameMode::ActivePlay,
    });
    assert_eq!(
        hub.read(),
        None,
        "a message without payment fields is unknown state"
    );

    let mut free = state(9, GameMode::ActivePlay);
    free.free_play = true;
    hub.apply(&free);
    assert!(!should_block_game_exit(true, hub.read()));
}

#[test]
fn credits_active_flag_or_paid_mode_block_close_game_only() {
    let hub = GameStateHub::default();
    hub.apply(&state(1, GameMode::Attract));
    assert!(should_block_game_exit(true, hub.read()));
    hub.apply(&state(0, GameMode::ActivePlay));
    assert!(should_block_game_exit(true, hub.read()));
    hub.apply(&state(0, GameMode::Attract));
    assert!(!should_block_game_exit(true, hub.read()));
    hub.apply(&GameState {
        credits: None,
        paid_play: Some(false),
        free_play: false,
        mode: GameMode::Ready,
    });
    assert!(
        should_block_game_exit(true, hub.read()),
        "a paid mode blocks on its own"
    );
    assert_eq!(hub.read().unwrap().mode, Some(GameMode::Ready));
}
