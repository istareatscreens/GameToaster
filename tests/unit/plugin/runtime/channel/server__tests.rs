use super::super::messages::{
    LogLevel, LogMessage, Message, Progress, ResultMessage, ResultStatus, Stage, MAX_LINE_BYTES,
};
use super::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
enum Seen {
    Connected,
    Message(Message),
    Invalid(Invalid),
    Ignored(String),
    Disconnected,
}

#[derive(Default)]
struct Recorder(Mutex<Vec<Seen>>);

impl Recorder {
    fn seen(&self) -> Vec<Seen> {
        self.0.lock().unwrap().clone()
    }

    fn wait_until(&self, condition: impl Fn(&[Seen]) -> bool) -> Vec<Seen> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let seen = self.seen();
            if condition(&seen) || Instant::now() > deadline {
                return seen;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl ChannelListener for Recorder {
    fn connected(&self, _client_pid: u32) {
        self.0.lock().unwrap().push(Seen::Connected);
    }

    fn message(&self, message: Message) {
        self.0.lock().unwrap().push(Seen::Message(message));
    }

    fn invalid(&self, reason: Invalid) {
        self.0.lock().unwrap().push(Seen::Invalid(reason));
    }

    fn ignored(&self, message_type: String) {
        self.0.lock().unwrap().push(Seen::Ignored(message_type));
    }

    fn disconnected(&self, _client_pid: u32) {
        self.0.lock().unwrap().push(Seen::Disconnected);
    }
}

fn operation_id(tag: u8) -> String {
    format!(
        "{:032x}",
        u128::from(std::process::id()) * 256 + u128::from(tag)
    )
}

fn connect(name: &str) -> std::fs::File {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match OpenOptions::new().write(true).open(name) {
            Ok(file) => return file,
            Err(error) if error.raw_os_error() == Some(231) && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("{name}: {error}"),
        }
    }
}

fn count(seen: &[Seen], wanted: &Seen) -> usize {
    seen.iter().filter(|item| *item == wanted).count()
}

#[test]
fn one_client_is_read_in_order_and_a_bad_line_and_an_unknown_type_are_each_told_once() {
    let recorder = Arc::new(Recorder::default());
    let mut server = ChannelServer::start(&operation_id(1), recorder.clone()).unwrap();
    {
        let mut client = connect(server.name());
        client
            .write_all(
                b"{\"type\":\"progress\",\"stage\":\"patching\",\"percent\":40}\n\
                  not json\n\
                  {\"type\":\"future\"}\n\
                  {\"type\":\"result\",\"status\":\"ok\",\"text\":\"done\"}\n",
            )
            .unwrap();
    }
    let seen = recorder.wait_until(|seen| seen.last() == Some(&Seen::Disconnected));
    assert_eq!(
        seen,
        [
            Seen::Connected,
            Seen::Message(Message::Progress(Progress {
                stage: Stage::Patching,
                percent: Some(40.0),
                text: String::new(),
            })),
            Seen::Invalid(Invalid::NotJson),
            Seen::Ignored("future".into()),
            Seen::Message(Message::Result(ResultMessage {
                status: ResultStatus::Ok,
                text: "done".into(),
            })),
            Seen::Disconnected,
        ]
    );
    server.shutdown();
}

#[test]
fn two_clients_at_once_both_deliver_and_an_overlong_line_is_dropped() {
    let recorder = Arc::new(Recorder::default());
    let mut server = ChannelServer::start(&operation_id(2), recorder.clone()).unwrap();
    let mut first = connect(server.name());
    let mut second = connect(server.name());
    let long = vec![b'a'; MAX_LINE_BYTES + 10];
    first.write_all(&long).unwrap();
    first
        .write_all(b"\n{\"type\":\"log\",\"level\":\"info\",\"text\":\"one\"}\n")
        .unwrap();
    second
        .write_all(b"{\"type\":\"log\",\"level\":\"info\",\"text\":\"two\"}\n")
        .unwrap();
    drop(first);
    drop(second);
    let seen = recorder.wait_until(|seen| count(seen, &Seen::Disconnected) == 2);
    assert_eq!(count(&seen, &Seen::Connected), 2);
    assert_eq!(count(&seen, &Seen::Invalid(Invalid::Overlong)), 1);
    for text in ["one", "two"] {
        assert!(seen.contains(&Seen::Message(Message::Log(LogMessage {
            level: LogLevel::Info,
            text: text.into(),
        }))));
    }
    server.shutdown();
}

#[test]
fn shutdown_is_prompt_with_no_client_and_with_an_idle_client() {
    let recorder = Arc::new(Recorder::default());
    let mut idle = ChannelServer::start(&operation_id(3), recorder.clone()).unwrap();
    let started = Instant::now();
    idle.shutdown();
    assert!(started.elapsed() < Duration::from_secs(2));

    let mut busy = ChannelServer::start(&operation_id(4), recorder.clone()).unwrap();
    let mut client = connect(busy.name());
    recorder.wait_until(|seen| seen.contains(&Seen::Connected));
    let started = Instant::now();
    busy.shutdown();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(
        client
            .write_all(b"{\"type\":\"future\"}\n")
            .and_then(|()| client.flush())
            .is_err(),
        "the server end is closed, so the client cannot write"
    );
    let seen = recorder.wait_until(|seen| seen.contains(&Seen::Disconnected));
    assert!(seen.contains(&Seen::Disconnected));
}

#[test]
fn lines_written_before_shutdown_are_delivered_even_when_shutdown_follows_at_once() {
    let expected = [
        Seen::Message(Message::Progress(Progress {
            stage: Stage::Complete,
            percent: Some(100.0),
            text: String::new(),
        })),
        Seen::Message(Message::Result(ResultMessage {
            status: ResultStatus::Failed,
            text: "no unpatched copy".into(),
        })),
    ];
    for round in 0..40 {
        let recorder = Arc::new(Recorder::default());
        let mut server =
            ChannelServer::start(&operation_id(100 + round), recorder.clone()).unwrap();
        {
            let mut client = connect(server.name());
            client
                .write_all(
                    b"{\"type\":\"progress\",\"stage\":\"complete\",\"percent\":100}\n\
                      {\"type\":\"result\",\"status\":\"failed\",\"text\":\"no unpatched copy\"}\n",
                )
                .unwrap();
        }
        server.shutdown();
        let messages: Vec<Seen> = recorder
            .seen()
            .into_iter()
            .filter(|seen| matches!(seen, Seen::Message(_)))
            .collect();
        assert_eq!(messages, expected, "round {round}");
    }
}

#[test]
fn a_name_already_in_use_cannot_be_served_twice() {
    let recorder = Arc::new(Recorder::default());
    let id = operation_id(5);
    let _first = ChannelServer::start(&id, recorder.clone()).unwrap();
    let error = ChannelServer::start(&id, recorder).unwrap_err();
    assert!(error.contains("already in use"), "{error}");
}

#[test]
fn the_invalid_line_budget_ends_the_connection() {
    let recorder = Arc::new(Recorder::default());
    let mut server = ChannelServer::start(&operation_id(6), recorder.clone()).unwrap();
    let mut client = connect(server.name());
    let garbage = "nope\n".repeat(1_000);
    let _ = client.write_all(garbage.as_bytes());
    let seen = recorder.wait_until(|seen| seen.contains(&Seen::Disconnected));
    assert!(
        seen.contains(&Seen::Disconnected),
        "the client was not dropped"
    );
    assert_eq!(
        count(&seen, &Seen::Invalid(Invalid::NotJson)),
        1,
        "a flood is reported once, not once per line"
    );
    drop(client);
    server.shutdown();
}

#[test]
fn unknown_types_are_told_once_per_connection_and_never_count_as_invalid() {
    let recorder = Arc::new(Recorder::default());
    let mut server = ChannelServer::start(&operation_id(7), recorder.clone()).unwrap();
    {
        let mut client = connect(server.name());
        client
            .write_all(
                b"{\"type\":\"heartbeat\",\"n\":1}\n\
                  {\"type\":\"heartbeat\",\"n\":2}\n\
                  {\"type\":\"telemetry_v3\"}\n\
                  {\"type\":\"log\",\"level\":\"info\",\"text\":\"still here\"}\n",
            )
            .unwrap();
    }
    let seen = recorder.wait_until(|seen| seen.last() == Some(&Seen::Disconnected));
    assert_eq!(
        seen,
        [
            Seen::Connected,
            Seen::Ignored("heartbeat".into()),
            Seen::Message(Message::Log(LogMessage {
                level: LogLevel::Info,
                text: "still here".into(),
            })),
            Seen::Disconnected,
        ]
    );
    server.shutdown();
}

#[test]
fn a_client_that_never_stops_writing_does_not_hold_shutdown() {
    let recorder = Arc::new(Recorder::default());
    let mut server = ChannelServer::start(&operation_id(8), recorder.clone()).unwrap();
    let mut client = connect(server.name());
    recorder.wait_until(|seen| seen.contains(&Seen::Connected));
    let flood = thread::spawn(move || {
        let line = b"{\"type\":\"log\",\"level\":\"debug\",\"text\":\"flood\"}\n";
        while client.write_all(line).is_ok() {}
    });
    recorder.wait_until(|seen| seen.iter().any(|item| matches!(item, Seen::Message(_))));
    let started = Instant::now();
    server.shutdown();
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "shutdown waited {:?} on a flooding client",
        started.elapsed()
    );
    flood.join().unwrap();
}
