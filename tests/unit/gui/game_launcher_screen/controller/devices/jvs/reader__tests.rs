use std::collections::VecDeque;
use std::sync::mpsc;

use super::super::host::JvsPlayer;
use super::super::protocol::{Frame, REPORT_OK, STATUS_OK};
use super::*;

fn ok(payload: &[u8]) -> Frame {
    Frame {
        destination: 0,
        payload: [&[STATUS_OK, REPORT_OK][..], payload].concat(),
    }
}

fn initialising() -> VecDeque<Result<Frame, String>> {
    VecDeque::from([
        Ok(ok(&[])),
        Ok(ok(b"BOARD\0")),
        Ok(ok(&[0x13])),
        Ok(ok(&[0x20])),
        Ok(ok(&[0x10])),
        Ok(ok(&[0x01, 1, 8, 0, 0])),
    ])
}

struct ScriptedLink {
    init: VecDeque<Result<Frame, String>>,
    polls: VecDeque<(&'static str, bool)>,
    cancelled: bool,
}

impl JvsLink for ScriptedLink {
    fn send_without_reply(&mut self, _request: &[u8]) -> Result<(), String> {
        Ok(())
    }

    fn transact(&mut self, _request: &[u8]) -> Result<Frame, String> {
        if let Some(reply) = self.init.pop_front() {
            return reply;
        }
        let (message, cancelled) = self
            .polls
            .pop_front()
            .expect("the reader polled past its script");
        self.cancelled = cancelled;
        Err(message.to_owned())
    }

    fn sense(&self) -> Result<Option<bool>, String> {
        Ok(None)
    }

    fn cancelled(&self) -> bool {
        self.cancelled
    }
}

fn read(mut link: ScriptedLink) -> (Vec<String>, usize) {
    let (sender, receiver) = mpsc::sync_channel(64);
    run(
        &JvsHost {
            player: JvsPlayer::P1,
        },
        &mut link,
        &EventSink::new(sender, None),
    );
    let events = receiver
        .try_iter()
        .map(|event| match event {
            SessionEvent::Connected => "connected".to_owned(),
            SessionEvent::Snapshot(_) => "snapshot".to_owned(),
            SessionEvent::Error(error) => format!("error: {error}"),
            SessionEvent::Disconnected => "disconnected".to_owned(),
        })
        .collect();
    (events, link.polls.len())
}

#[test]
fn a_failed_poll_on_a_cancelled_link_ends_the_reader_without_an_error() {
    let (events, unread) = read(ScriptedLink {
        init: initialising(),
        polls: VecDeque::from([("JVS read timed out", true)]),
        cancelled: false,
    });
    assert_eq!(events, ["connected", "disconnected"]);
    assert_eq!(unread, 0);
}

#[test]
fn a_cancelled_message_on_a_live_link_is_reported_and_polling_continues() {
    let (events, unread) = read(ScriptedLink {
        init: initialising(),
        polls: VecDeque::from([("JVS read cancelled", false), ("JVS read timed out", true)]),
        cancelled: false,
    });
    assert_eq!(
        events,
        [
            "connected",
            "error: JVS poll error #1: JVS read cancelled",
            "disconnected"
        ]
    );
    assert_eq!(
        unread, 0,
        "the reader polled again after the reported failure"
    );
}

#[test]
fn a_failed_initialisation_is_reported_then_disconnected() {
    let (events, _) = read(ScriptedLink {
        init: VecDeque::from([Err("JVS assign address failed".to_owned())]),
        polls: VecDeque::new(),
        cancelled: false,
    });
    assert_eq!(events, ["error: JVS assign address failed", "disconnected"]);
}

#[test]
fn a_failed_poll_is_reported_the_first_time_and_at_each_power_of_two() {
    let reported: Vec<u64> = (1..=10)
        .filter(|count| should_report_error(*count))
        .collect();
    assert_eq!(reported, vec![1, 2, 4, 8]);
}
