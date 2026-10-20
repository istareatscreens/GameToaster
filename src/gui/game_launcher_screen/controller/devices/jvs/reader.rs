use std::thread;
use std::time::Duration;

use super::super::super::framework::events::{EventSink, SessionEvent};
use super::host::JvsHost;
use super::link::JvsLink;

const POLL_INTERVAL: Duration = Duration::from_millis(4);

pub(super) fn run(host: &JvsHost, link: &mut dyn JvsLink, events: &EventSink) {
    if let Err(error) = host.initialize(link) {
        let _ = events.try_send(SessionEvent::Error(error));
        let _ = events.try_send(SessionEvent::Disconnected);
        return;
    }
    let _ = events.try_send(SessionEvent::Connected);
    let mut errors = 0u64;
    loop {
        match host.poll(link) {
            Ok(snapshot) => {
                errors = 0;
                let _ = events.try_send(SessionEvent::Snapshot(snapshot));
            }
            Err(_) if link.cancelled() => break,
            Err(error) => {
                errors += 1;
                if should_report_error(errors) {
                    let _ = events.try_send(SessionEvent::Error(format!(
                        "JVS poll error #{errors}: {error}"
                    )));
                }
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
    let _ = events.try_send(SessionEvent::Disconnected);
}

fn should_report_error(count: u64) -> bool {
    count == 1 || count.is_power_of_two()
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/jvs/reader__tests.rs"]
mod tests;
