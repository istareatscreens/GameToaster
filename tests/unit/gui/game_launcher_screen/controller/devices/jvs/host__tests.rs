use std::cell::RefCell;
use std::collections::VecDeque;

use super::super::protocol::decode;
use super::*;

const BOARD_IDENTITY_OBF: &str = "Hp0z0MZkg7+Zx9nC0Vgif5YAZ8br8Nm+y0ep";

fn deobf_b64(b64: &str) -> Vec<u8> {
    const KEY: [u8; 16] = [
        0x4D, 0x46, 0x48, 0x4B, 0x9E, 0x37, 0x79, 0xB9, 0x2C, 0x1B, 0xA5, 0x5A, 0xF0, 0x0D, 0xC3,
        0x6E,
    ];

    fn b64_val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for &c in b64.as_bytes() {
        let Some(v) = b64_val(c) else { continue };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    for (i, byte) in out.iter_mut().enumerate() {
        *byte ^= KEY[i % 16] ^ ((i as u32).wrapping_mul(0x9E) & 0xFF) as u8;
    }
    out
}

struct ScriptedLink {
    endpoint: &'static str,
    responses: VecDeque<Frame>,
    requests: Vec<Vec<u8>>,
    senses: RefCell<VecDeque<Option<bool>>>,
}

impl JvsLink for ScriptedLink {
    fn send_without_reply(&mut self, request: &[u8]) -> Result<(), String> {
        self.requests.push(request.to_vec());
        Ok(())
    }

    fn transact(&mut self, request: &[u8]) -> Result<Frame, String> {
        self.requests.push(request.to_vec());
        self.responses
            .pop_front()
            .ok_or_else(|| "script has no response".to_string())
    }

    fn sense(&self) -> Result<Option<bool>, String> {
        Ok(self.senses.borrow_mut().pop_front().flatten())
    }

    fn cancelled(&self) -> bool {
        false
    }
}

fn ok(data: &[u8]) -> Frame {
    Frame {
        destination: 0,
        payload: [vec![STATUS_OK, REPORT_OK], data.to_vec()].concat(),
    }
}

#[test]
fn scripted_handshake_requires_switch_capability_and_polls_one_request() {
    let mut link = ScriptedLink {
        endpoint: "COM4",
        responses: VecDeque::from([
            ok(&[]),
            ok(&deobf_b64(BOARD_IDENTITY_OBF)),
            ok(&[0x13]),
            ok(&[0x20]),
            ok(&[0x10]),
            ok(&[0x01, 2, 14, 0, 0]),
            Frame {
                destination: 0,
                payload: vec![1, 1, 0, 0x0D, 0xD8],
            },
        ]),
        requests: Vec::new(),
        senses: RefCell::new(VecDeque::from([Some(true), Some(false)])),
    };
    let host = JvsHost {
        player: JvsPlayer::P1,
    };
    host.initialize(&mut link).unwrap();
    assert!(
        link.senses.borrow().is_empty(),
        "the line is sensed after reset and assignment"
    );
    assert_eq!(link.requests.len(), 7);
    let snapshot = host.poll(&mut link).unwrap();
    assert!(snapshot.contains(crate::game_launcher_screen::controller::Button::Ba6));
    assert_eq!(
        decode(link.requests.last().unwrap()).unwrap().payload,
        vec![0x20, 1, 2]
    );
    assert_eq!(link.endpoint, "COM4");
}

#[test]
fn read_coins_issues_read_coin_and_decodes_slots() {
    use crate::gui::common::coin::{CoinMonitor, CoinProvider, CoinReading, CoinSlot};

    let coin_reply = |count: u16| Frame {
        destination: 0,
        payload: vec![
            STATUS_OK,
            REPORT_OK,
            ((count >> 8) & 0x3F) as u8,
            (count & 0xFF) as u8,
        ],
    };
    let mut link = ScriptedLink {
        endpoint: "COM4",
        responses: VecDeque::from([coin_reply(0), coin_reply(2)]),
        requests: Vec::new(),
        senses: RefCell::new(VecDeque::new()),
    };
    let host = JvsHost {
        player: JvsPlayer::P1,
    };

    let first = host.read_coins(&mut link, 1).unwrap();
    assert_eq!(
        decode(link.requests.last().unwrap()).unwrap().payload,
        vec![0x21, 1]
    );
    assert_eq!(CoinSlot::from_jvs(first[0].0, first[0].1).count, 0);
    let second = host.read_coins(&mut link, 1).unwrap();
    assert_eq!(CoinSlot::from_jvs(second[0].0, second[0].1).count, 2);

    struct PairProvider(std::cell::RefCell<VecDeque<Vec<(u8, u8)>>>);
    impl CoinProvider for PairProvider {
        fn name(&self) -> &str {
            "jvs-sim"
        }
        fn poll(&mut self) -> Option<CoinReading> {
            self.0.borrow_mut().pop_front().map(|pairs| CoinReading {
                slots: pairs
                    .into_iter()
                    .map(|(hi, lo)| CoinSlot::from_jvs(hi, lo))
                    .collect(),
            })
        }
        fn set_accept(&mut self, _accept: bool) -> bool {
            false
        }
    }
    let mut monitor = CoinMonitor::new(Box::new(PairProvider(std::cell::RefCell::new(
        VecDeque::from([first, second]),
    ))));
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 2);
}

#[test]
fn missing_switch_feature_is_rejected() {
    assert!(switch_capability(&[0x02, 2, 0, 0, 0]).is_err());
    assert!(switch_capability(&[0x01, 1, 4, 0, 0]).is_err());
    assert!(switch_capability(&[0x01, 1, 8, 0, 0]).is_ok());
}

#[test]
fn both_verified_endpoint_profiles_use_the_same_bounded_host_transcript() {
    for endpoint in ["COM4", r"\\.\mxjvs"] {
        let mut link = ScriptedLink {
            endpoint,
            responses: VecDeque::from([
                ok(&[]),
                ok(&deobf_b64(BOARD_IDENTITY_OBF)),
                ok(&[0x13]),
                ok(&[0x20]),
                ok(&[0x10]),
                ok(&[0x01, 1, 8, 0, 0]),
            ]),
            requests: Vec::new(),
            senses: RefCell::new(VecDeque::from([None, None])),
        };
        JvsHost {
            player: JvsPlayer::P1,
        }
        .initialize(&mut link)
        .unwrap();
        assert_eq!(link.requests.len(), 7, "{endpoint}");
        assert_eq!(link.endpoint, endpoint);
    }
}
