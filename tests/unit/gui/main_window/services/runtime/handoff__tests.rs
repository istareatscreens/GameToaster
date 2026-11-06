use super::*;
use std::thread;

#[test]
fn the_worker_waits_for_the_answer_and_gets_it() {
    let gate = HandoffGate::new();
    let answering = gate.clone();
    let worker = thread::spawn(move || gate.request(&|| false));

    answering.answer(Ok(()));
    assert_eq!(worker.join().unwrap(), Ok(()));
}

#[test]
fn an_answer_that_arrives_first_is_still_the_answer() {
    let gate = HandoffGate::new();
    gate.answer(Ok(()));
    assert_eq!(gate.request(&|| false), Ok(()));
}

#[test]
fn a_refused_release_is_reported_to_the_worker_verbatim() {
    let gate = HandoffGate::new();
    gate.answer(Err("2 handles, 1 reader".into()));
    assert_eq!(gate.request(&|| false), Err("2 handles, 1 reader".into()));
}

#[test]
fn the_first_answer_wins() {
    let gate = HandoffGate::new();
    gate.answer(Ok(()));
    gate.answer(Err("too late".into()));
    assert_eq!(gate.request(&|| false), Ok(()));
}

#[test]
fn a_launch_cancelled_while_waiting_stops_the_wait_and_never_starts_the_game() {
    let gate = HandoffGate::new();

    let outcome = gate.request(&|| true);
    assert!(
        outcome.is_err(),
        "a cancelled launch must not be told to start"
    );
}
