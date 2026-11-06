use super::*;

const GAME: GameId = GameId(3);
const OTHER: GameId = GameId(8);

fn flight(worker: Option<(GameId, bool)>, awaiting: Option<GameId>) -> Flight {
    Flight { worker, awaiting }
}

#[test]
fn nothing_in_flight_offers_no_cancel() {
    let idle = Flight::default();
    assert_eq!(idle.cancel_action(), CancelAction::Nothing);
    assert!(!idle.cancellable());
    assert!(idle.follows_stages());
}

#[test]
fn a_launch_the_worker_runs_is_cancelled_through_its_token() {
    let running = flight(Some((GAME, false)), None);
    assert_eq!(running.cancel_action(), CancelAction::CancelWorker(GAME));
    assert!(running.cancellable());
    assert!(running.follows_stages());
    assert!(!running.close_when_launched(GAME));
}

#[test]
fn a_requested_cancel_is_not_requested_twice_and_keeps_its_label() {
    let cancelling = flight(Some((GAME, true)), None);
    assert_eq!(cancelling.cancel_action(), CancelAction::Nothing);
    assert!(
        !cancelling.cancellable(),
        "the control goes while the cancel runs"
    );
    assert!(!cancelling.follows_stages());
}

#[test]
fn a_launch_cancelled_too_late_for_the_worker_is_closed_when_it_reports_launched() {
    let cancelling = flight(Some((GAME, true)), None);
    assert!(cancelling.close_when_launched(GAME));
    assert!(!cancelling.close_when_launched(OTHER));
    assert!(!flight(Some((GAME, false)), None).close_when_launched(GAME));
}

#[test]
fn an_awaited_program_is_cancelled_by_closing_its_game() {
    let awaiting = flight(None, Some(GAME));
    assert_eq!(awaiting.cancel_action(), CancelAction::CloseGame(GAME));
    assert!(awaiting.cancellable());
}

#[test]
fn the_worker_phase_comes_first_when_both_are_known() {
    let both = flight(Some((OTHER, false)), Some(GAME));
    assert_eq!(both.cancel_action(), CancelAction::CancelWorker(OTHER));
    let cancelling = flight(Some((OTHER, true)), Some(GAME));
    assert_eq!(cancelling.cancel_action(), CancelAction::Nothing);
}
