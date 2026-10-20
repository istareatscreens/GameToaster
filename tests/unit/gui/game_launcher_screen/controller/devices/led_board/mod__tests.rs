use std::cell::RefCell;
use std::rc::Rc;

use super::*;

const REFERENCE: [&[u8]; 6] = [
    &[0xE0, 0x11, 0x01, 0x01, 0x10, 0x23],
    &[0xE0, 0x11, 0x01, 0x01, 0x10, 0x23],
    &[0xE0, 0x11, 0x01, 0x01, 0x10, 0x23],
    &[
        0xE0, 0x11, 0x01, 0x08, 0x32, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6C,
    ],
    &[0xE0, 0x11, 0x01, 0x04, 0x39, 0x00, 0x00, 0x00, 0x4F],
    &[0xE0, 0x11, 0x01, 0x01, 0x3C, 0x4F],
];

#[test]
fn the_sequence_is_the_reference_byte_for_byte_with_the_boards_pauses() {
    let sequence = turn_off_sequence();
    let frames: Vec<&[u8]> = sequence.iter().map(|step| step.bytes.as_slice()).collect();
    assert_eq!(frames, REFERENCE);
    let pauses: Vec<u64> = sequence
        .iter()
        .map(|step| step.pause_after.as_millis() as u64)
        .collect();
    assert_eq!(pauses, [50, 50, 150, 10, 10, 50]);
}

type Journal = Rc<RefCell<Vec<String>>>;

struct Recorder {
    journal: Journal,
    fail_on_write: Option<usize>,
    writes: usize,
}

impl LightPort for Recorder {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.writes += 1;
        if self.fail_on_write == Some(self.writes) {
            return Err("write timed out".into());
        }
        self.journal.borrow_mut().push(format!("{bytes:02X?}"));
        Ok(())
    }
}

fn run(paths: &[&str], busy: &[&str], failing: &[&str]) -> (Result<(), LightsError>, Vec<String>) {
    let journal: Journal = Rc::default();
    let paths: Vec<String> = paths.iter().map(|path| (*path).to_owned()).collect();
    let opened = Rc::clone(&journal);
    let paused = Rc::clone(&journal);
    let result = turn_off_ports(
        &paths,
        |path| {
            opened.borrow_mut().push(format!("open {path}"));
            if busy.contains(&path) {
                Err(SerialOpenError::InUse)
            } else if failing.contains(&path) {
                Err(SerialOpenError::Failed(format!(
                    "failed to open serial port {path}"
                )))
            } else {
                Ok(Box::new(Recorder {
                    journal: Rc::clone(&opened),
                    fail_on_write: None,
                    writes: 0,
                }) as Box<dyn LightPort>)
            }
        },
        |duration| {
            paused
                .borrow_mut()
                .push(format!("pause {}", duration.as_millis()))
        },
    );
    let journal = journal.borrow().clone();
    (result, journal)
}

#[test]
fn every_port_receives_the_whole_sequence_with_its_pauses() {
    let (result, journal) = run(&[r"\\.\COM5", r"\\.\COM6"], &[], &[]);
    assert_eq!(result, Ok(()));
    let mut expected = Vec::new();
    for path in [r"\\.\COM5", r"\\.\COM6"] {
        expected.push(format!("open {path}"));
        for step in turn_off_sequence() {
            expected.push(format!("{:02X?}", step.bytes));
            expected.push(format!("pause {}", step.pause_after.as_millis()));
        }
    }
    assert_eq!(journal, expected);
}

#[test]
fn a_port_held_by_another_program_makes_the_whole_attempt_busy() {
    let (result, journal) = run(&[r"\\.\COM5", r"\\.\COM6"], &[r"\\.\COM5"], &[]);
    assert_eq!(result, Err(LightsError::Busy));
    assert!(
        journal.contains(&r"open \\.\COM6".to_owned()),
        "the free port was still turned off"
    );
}

#[test]
fn a_failing_port_does_not_stop_the_other_and_is_named() {
    let (result, journal) = run(&[r"\\.\COM5", r"\\.\COM6"], &[], &[r"\\.\COM5"]);
    assert_eq!(
        result,
        Err(LightsError::Failed(
            r"failed to open serial port \\.\COM5".into()
        ))
    );
    assert_eq!(
        journal.iter().filter(|line| line.starts_with('[')).count(),
        6,
        "COM6 got all six frames"
    );
}

#[test]
fn a_write_that_fails_ends_that_port_and_names_it() {
    let journal: Journal = Rc::default();
    let paths = vec![r"\\.\COM21".to_owned()];
    let opened = Rc::clone(&journal);
    let result = turn_off_ports(
        &paths,
        |_| {
            Ok(Box::new(Recorder {
                journal: Rc::clone(&opened),
                fail_on_write: Some(4),
                writes: 0,
            }) as Box<dyn LightPort>)
        },
        |_| {},
    );
    assert_eq!(
        result,
        Err(LightsError::Failed(r"\\.\COM21: write timed out".into()))
    );
    assert_eq!(
        journal.borrow().len(),
        3,
        "the three resets, then the failed write"
    );
}

#[test]
fn ports_are_comma_separated_com_names_each_kept_once() {
    let _locale = crate::gui::common::harness::locale("en");
    assert_eq!(port_paths("COM21").unwrap(), [r"\\.\COM21"]);
    assert_eq!(
        port_paths(" COM5, com6 ,COM5,").unwrap(),
        [r"\\.\COM5", r"\\.\COM6"]
    );
    assert_eq!(port_paths("").unwrap(), Vec::<String>::new());
    let error = port_paths("COM5, LED").unwrap_err();
    assert_eq!(
        error,
        rust_i18n::t!("controller.validation.led_port", port => "LED").to_string()
    );
    assert!(LIGHTS.validate_ports("COM5,COM6").is_ok());
    assert!(LIGHTS.validate_ports("COM0").is_err());
    assert_eq!(LIGHTS.name(), "led-board");
}

#[test]
fn a_port_that_does_not_exist_fails_without_opening_anything_else() {
    let result = LIGHTS.turn_off("COM255");
    assert!(matches!(result, Err(LightsError::Failed(_))), "{result:?}");
}

fn hardware_port() -> Option<String> {
    let port = std::env::var("GAMETOASTER_LED_TEST_PORT").ok()?;
    let port = port.trim().to_owned();
    (!port.is_empty()).then_some(port)
}

#[test]
#[ignore = "needs an LED board on the port in GAMETOASTER_LED_TEST_PORT; run with --include-ignored"]
fn the_board_on_a_real_port_turns_its_lights_off() {
    let Some(port) = hardware_port() else {
        eprintln!("GAMETOASTER_LED_TEST_PORT is not set: no LED board to drive");
        return;
    };
    assert_eq!(LIGHTS.turn_off(&port), Ok(()));
}

#[test]
#[ignore = "needs an LED board on the port in GAMETOASTER_LED_TEST_PORT; run with --include-ignored"]
fn a_real_port_held_by_another_handle_is_busy_until_it_is_released() {
    let Some(port) = hardware_port() else {
        eprintln!("GAMETOASTER_LED_TEST_PORT is not set: no LED board to drive");
        return;
    };
    let audit = ResourceAudit::new();
    let path = com_port_path(&port).expect("a COM port name");
    let held = SerialTransport::open(&path, &audit).expect("the test port opens");

    assert_eq!(LIGHTS.turn_off(&port), Err(LightsError::Busy));

    drop(held);
    assert_eq!(LIGHTS.turn_off(&port), Ok(()));
}

#[test]
#[ignore = "needs an LED board on the port in GAMETOASTER_LED_TEST_PORT; run with --include-ignored"]
fn the_job_waits_for_a_real_port_to_be_released_and_then_turns_the_lights_off() {
    use super::super::super::framework::lights::job::{LightsOffJob, LightsOutcome, LightsTiming};

    let Some(port) = hardware_port() else {
        eprintln!("GAMETOASTER_LED_TEST_PORT is not set: no LED board to drive");
        return;
    };
    let audit = ResourceAudit::new();
    let path = com_port_path(&port).expect("a COM port name");

    let held = SerialTransport::open(&path, &audit).expect("the test port opens");
    let holder = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        drop(held);
    });
    let (sender, outcome) = std::sync::mpsc::channel();
    let mut job = LightsOffJob::default();
    let timing = LightsTiming {
        settle: Duration::ZERO,
        retry_every: Duration::from_millis(100),
        give_up_after: Duration::from_secs(5),
    };

    let started = job
        .start(&LIGHTS, port, timing, move |result| {
            let _ = sender.send(result);
        })
        .unwrap();

    assert!(started);
    let result = outcome.recv_timeout(Duration::from_secs(10)).unwrap();
    holder.join().unwrap();
    let LightsOutcome::TurnedOff { attempts } = result else {
        panic!("expected the lights off, got {result:?}");
    };
    assert!(attempts > 1, "the port was never busy ({attempts} attempt)");
}
