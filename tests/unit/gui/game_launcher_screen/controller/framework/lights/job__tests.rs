use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::Mutex;

use super::*;

struct Scripted {
    results: Mutex<VecDeque<Result<(), LightsError>>>,
    calls: Mutex<Vec<String>>,
}

impl Scripted {
    fn new(results: Vec<Result<(), LightsError>>) -> &'static Self {
        Box::leak(Box::new(Self {
            results: Mutex::new(results.into()),
            calls: Mutex::new(Vec::new()),
        }))
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl ControllerLights for Scripted {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn validate_ports(&self, _ports: &str) -> Result<(), String> {
        Ok(())
    }

    fn turn_off(&self, ports: &str) -> Result<(), LightsError> {
        self.calls.lock().unwrap().push(ports.to_owned());
        let mut results = self.results.lock().unwrap();
        if results.len() > 1 {
            results.pop_front().unwrap()
        } else {
            results.front().cloned().unwrap()
        }
    }
}

const FAST: LightsTiming = LightsTiming {
    settle: Duration::ZERO,
    retry_every: Duration::from_millis(5),
    give_up_after: Duration::from_millis(60),
};

fn start(
    job: &mut LightsOffJob,
    lights: &'static Scripted,
    timing: LightsTiming,
) -> mpsc::Receiver<LightsOutcome> {
    let (sender, outcome) = mpsc::channel();
    let started = job
        .start(lights, "COM21".into(), timing, move |result| {
            let _ = sender.send(result);
        })
        .unwrap();
    assert!(started, "no job was running");
    outcome
}

fn wait_for(outcome: &mpsc::Receiver<LightsOutcome>) -> LightsOutcome {
    outcome
        .recv_timeout(Duration::from_secs(5))
        .expect("the job reports its outcome")
}

#[test]
fn the_shipped_timing_settles_retries_and_gives_up_within_bounds() {
    assert_eq!(TIMING.settle, Duration::from_millis(500));
    assert_eq!(TIMING.retry_every, Duration::from_millis(250));
    assert_eq!(TIMING.give_up_after, Duration::from_secs(15));
}

#[test]
fn a_busy_port_is_retried_until_the_game_releases_it() {
    let lights = Scripted::new(vec![Err(LightsError::Busy), Err(LightsError::Busy), Ok(())]);
    let mut job = LightsOffJob::default();

    let outcome = wait_for(&start(&mut job, lights, FAST));

    assert_eq!(outcome, LightsOutcome::TurnedOff { attempts: 3 });
    assert_eq!(lights.calls(), ["COM21", "COM21", "COM21"]);
}

#[test]
fn a_port_that_stays_busy_is_given_up_after_the_bound() {
    let lights = Scripted::new(vec![Err(LightsError::Busy)]);
    let mut job = LightsOffJob::default();
    let started = Instant::now();

    let outcome = wait_for(&start(&mut job, lights, FAST));

    let LightsOutcome::GaveUp { attempts } = outcome else {
        panic!("expected GaveUp, got {outcome:?}");
    };
    assert!(attempts >= 2, "retried only {attempts} times");
    assert_eq!(lights.calls().len(), attempts as usize);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn a_failure_other_than_busy_ends_the_job_at_once() {
    let lights = Scripted::new(vec![Err(LightsError::Failed("no such port".into()))]);
    let mut job = LightsOffJob::default();

    let outcome = wait_for(&start(&mut job, lights, FAST));

    assert_eq!(outcome, LightsOutcome::Failed("no such port".into()));
    assert_eq!(lights.calls().len(), 1);
}

#[test]
fn a_launch_during_the_settle_cancels_before_any_port_is_opened() {
    let lights = Scripted::new(vec![Ok(())]);
    let mut job = LightsOffJob::default();
    let slow = LightsTiming {
        settle: Duration::from_secs(10),
        ..FAST
    };
    let outcome = start(&mut job, lights, slow);

    job.cancel();

    assert_eq!(wait_for(&outcome), LightsOutcome::Cancelled);
    assert!(
        lights.calls().is_empty(),
        "a port was opened after the cancel"
    );
}

#[test]
fn a_launch_while_the_port_is_busy_cancels_the_retries() {
    let lights = Scripted::new(vec![Err(LightsError::Busy)]);
    let mut job = LightsOffJob::default();
    let patient = LightsTiming {
        give_up_after: Duration::from_secs(10),
        ..FAST
    };
    let outcome = start(&mut job, lights, patient);
    while lights.calls().is_empty() {
        thread::sleep(Duration::from_millis(1));
    }

    job.cancel();

    assert_eq!(wait_for(&outcome), LightsOutcome::Cancelled);
}

#[test]
fn only_one_job_runs_at_a_time() {
    let lights = Scripted::new(vec![Ok(())]);
    let mut job = LightsOffJob::default();
    let slow = LightsTiming {
        settle: Duration::from_secs(10),
        ..FAST
    };
    let first = start(&mut job, lights, slow);
    assert!(job.is_running());

    let second = job.start(lights, "COM21".into(), FAST, |_| panic!("a second job ran"));
    assert_eq!(second, Ok(false));

    job.cancel();
    assert_eq!(wait_for(&first), LightsOutcome::Cancelled);
    while job.is_running() {
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        wait_for(&start(&mut job, lights, FAST)),
        LightsOutcome::TurnedOff { attempts: 1 }
    );
}

#[test]
fn dropping_the_job_stops_a_waiting_thread() {
    let lights = Scripted::new(vec![Ok(())]);
    let mut job = LightsOffJob::default();
    let slow = LightsTiming {
        settle: Duration::from_secs(10),
        ..FAST
    };
    let outcome = start(&mut job, lights, slow);

    drop(job);

    assert_eq!(wait_for(&outcome), LightsOutcome::Cancelled);
    assert!(lights.calls().is_empty());
}
