use super::*;
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

fn candidate(vid: u16, pid: u16, len: usize) -> HidCandidate {
    HidCandidate {
        path: vec![0],
        vid,
        pid,
        input_report_len: len,
    }
}

#[test]
fn selection_is_exact_validates_length_and_rejects_ambiguity() {
    let wanted = HidIdentity {
        vid: 0x0CA3,
        pid: 0x0021,
        minimum_input_report_len: 31,
    };
    assert_eq!(
        select_matching(vec![candidate(0x0CA3, 0x0021, 64)], &wanted)
            .unwrap()
            .input_report_len,
        64
    );
    assert!(select_matching(vec![candidate(0x0CA3, 0x0021, 30)], &wanted).is_err());
    assert!(select_matching(
        vec![candidate(0x0CA3, 0x0021, 64), candidate(0x0CA3, 0x0021, 64)],
        &wanted,
    )
    .unwrap_err()
    .contains("multiple"));
}

#[test]
fn the_wanted_identity_is_what_was_entered_and_the_drivers_own_where_emptied() {
    use super::super::super::devices::{adx_hid, adx_io4};

    let io4 = adx_io4::adapter();
    let mut config = io4.descriptor().default_config();
    let identity = |vid: u16, pid: u16| HidIdentity {
        vid,
        pid,
        minimum_input_report_len: 64,
    };
    assert_eq!(io4.wanted_identity(&config), Ok(identity(0x0CA3, 0x0021)));

    config.options.insert("vid".into(), String::new());
    config.options.insert("pid".into(), "  ".into());
    assert_eq!(io4.wanted_identity(&config), Ok(identity(0x0CA3, 0x0021)));

    config.options.insert("vid".into(), "1234".into());
    assert_eq!(io4.wanted_identity(&config), Ok(identity(0x1234, 0x0021)));

    config.options.insert("pid".into(), "zz".into());
    assert!(io4.wanted_identity(&config).is_err());

    let ring = adx_hid::adapter();
    let mut config = ring.descriptor().default_config();
    config.options.insert("vid".into(), String::new());
    config.options.insert("pid".into(), String::new());
    assert_eq!(
        ring.wanted_identity(&config),
        Ok(HidIdentity {
            vid: 0x2E3C,
            pid: 0x5750,
            minimum_input_report_len: 14,
        })
    );
}

#[test]
fn blocked_reader_cancels_joins_and_releases_every_tracked_resource() {
    let audit = ResourceAudit::new();
    let event = || {
        Arc::new(
            TrackedHandle::new(
                unsafe { CreateEventW(None, true, false, None).unwrap() },
                &audit,
            )
            .unwrap(),
        )
    };
    let device = event();
    let cancellation = event();
    let read_event = event();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let blocked_on = cancellation.clone();
    let thread_audit = audit.clone();
    let reader = thread::spawn(move || {
        let _guard = ReaderGuard::new(&thread_audit);
        unsafe {
            let _ = WaitForSingleObject(blocked_on.raw(), INFINITE);
        }
        let _ = done_tx.send(());
    });
    let mut session = HidSession {
        device: Some(device),
        cancellation: Some(cancellation),
        read_event: Some(read_event),
        reader: Some(reader),
        done: done_rx,
    };
    session.disconnect(Duration::from_secs(1)).unwrap();
    assert_eq!(audit.counts(), (0, 0));
}

fn unit(path: u16, vid: u16, pid: u16, len: usize) -> HidCandidate {
    HidCandidate {
        path: vec![path],
        vid,
        pid,
        input_report_len: len,
    }
}

const BY_SETTING: HidUnits = HidUnits {
    feature_byte: 4,
    settings: &[1, 2],
};

fn unit_identity() -> HidIdentity {
    HidIdentity {
        vid: 0x0E8F,
        pid: 0x1224,
        minimum_input_report_len: 8,
    }
}

#[test]
fn units_are_picked_by_their_own_setting_one_of_each_in_setting_order() {
    let candidates = vec![
        unit(1, 0x0E8F, 0x1224, 8),
        unit(2, 0x0E8F, 0x1224, 8),
        unit(3, 0x0E8F, 0x1224, 8),
        unit(4, 0x0E8F, 0x1224, 4),
        unit(5, 0x1234, 0x1224, 8),
    ];
    let settings = [(1, 2), (2, 1), (3, 1), (4, 2), (5, 1)];
    let setting_of = |candidate: &HidCandidate| {
        settings
            .iter()
            .find(|(path, _)| candidate.path == vec![*path])
            .map(|&(_, setting)| setting)
    };
    let chosen = select_units(candidates, &unit_identity(), BY_SETTING, setting_of).unwrap();
    let paths: Vec<Vec<u16>> = chosen.iter().map(|unit| unit.path.clone()).collect();
    assert_eq!(paths, vec![vec![2], vec![1]]);
}

#[test]
fn a_lone_unit_is_read_whichever_player_it_is_set_to() {
    for setting in [1u8, 2] {
        let chosen = select_units(
            vec![unit(7, 0x0E8F, 0x1224, 8)],
            &unit_identity(),
            BY_SETTING,
            |_| Some(setting),
        )
        .unwrap();
        assert_eq!(chosen.len(), 1);
    }
}

#[test]
fn an_interface_without_a_setting_is_never_opened_in_a_units_place() {
    for setting in [None, Some(0u8), Some(3u8)] {
        let error = select_units(
            vec![unit(8, 0x0E8F, 0x1224, 8)],
            &unit_identity(),
            BY_SETTING,
            |_| setting,
        )
        .unwrap_err();
        assert!(error.contains("1P or 2P"), "{error}");
    }
    assert!(select_units(Vec::new(), &unit_identity(), BY_SETTING, |_| Some(1)).is_err());
}

fn seen(event: SessionEvent) -> String {
    match event {
        SessionEvent::Connected => "connected".into(),
        SessionEvent::Snapshot(word) => format!("snapshot {:#06x}", word.bits()),
        SessionEvent::Error(message) => format!("error {message}"),
        SessionEvent::Disconnected => "disconnected".into(),
    }
}

#[test]
fn the_units_are_read_as_one_ring() {
    let (sender, receiver) = mpsc::sync_channel(16);
    let mut merge = UnitMerge::new(EventSink::new(sender, None), 2);
    let word = ButtonWord::from_bits;
    merge.started();
    merge.started();
    merge.event(0, SessionEvent::Snapshot(word(0b0010)));
    merge.event(1, SessionEvent::Snapshot(word(0b0100)));
    merge.event(0, SessionEvent::Snapshot(word(0)));
    merge.event(1, SessionEvent::Error("read failed".into()));
    merge.stopped(1);
    merge.stopped(0);
    let events: Vec<String> = receiver.try_iter().map(seen).collect();
    assert_eq!(
        events,
        vec![
            "connected",
            "snapshot 0x0002",
            "snapshot 0x0006",
            "snapshot 0x0004",
            "error read failed",
            "snapshot 0x0000",
            "disconnected",
        ]
    );
}

#[test]
fn the_dao_driver_reads_its_units_by_setting_and_the_rings_read_one_device() {
    use super::super::super::devices::{adx_hid, adx_io4, dao, ndx_hid, ndx_io4};

    assert_eq!(dao::adapter().units, Some(BY_SETTING));
    for ring in [
        adx_hid::adapter(),
        adx_io4::adapter(),
        ndx_hid::adapter(),
        ndx_io4::adapter(),
    ] {
        assert_eq!(ring.units, None);
    }
}
