use super::*;

#[test]
fn framing_round_trips_escaped_bytes_and_rejects_corruption() {
    let wire = encode(1, &[0x10, SYNC, MARK]).unwrap();
    assert!(wire.windows(2).any(|pair| pair == [MARK, SYNC - 1]));
    assert_eq!(decode(&wire).unwrap().payload, vec![0x10, SYNC, MARK]);
    let mut corrupt = wire;
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(decode(&corrupt).unwrap_err().contains("checksum"));
}

#[test]
fn read_switch_request_is_exact_and_switch_bits_decode() {
    let request = decode(&read_switches_request(1, 1).unwrap()).unwrap();
    assert_eq!(
        request,
        Frame {
            destination: 1,
            payload: vec![0x20, 1, 2]
        }
    );
    let ba6 = SWITCH_IDLE & !(1 << 5);
    let frame = Frame {
        destination: 0,
        payload: vec![STATUS_OK, REPORT_OK, 0, (ba6 >> 8) as u8, ba6 as u8],
    };
    assert_eq!(
        decode_switch_response(&frame, 0).unwrap().bits(),
        Button::Ba6.mask()
    );
}

#[test]
fn player_two_switches_decode_from_the_second_field() {
    let request = decode(&read_switches_request(1, 2).unwrap()).unwrap();
    assert_eq!(request.payload, vec![0x20, 2, 2]);
    let p2_ba4 = SWITCH_IDLE & !(1 << 7);
    let frame = Frame {
        destination: 0,
        payload: vec![
            STATUS_OK,
            REPORT_OK,
            0,
            (SWITCH_IDLE >> 8) as u8,
            SWITCH_IDLE as u8,
            (p2_ba4 >> 8) as u8,
            p2_ba4 as u8,
        ],
    };

    assert_eq!(decode_switch_response(&frame, 0).unwrap().bits(), 0);
    assert_eq!(
        decode_switch_response(&frame, 1).unwrap().bits(),
        Button::Ba4.mask()
    );
}

#[test]
fn read_coin_request_is_exact_and_response_decodes_to_slot_pairs() {
    let request = decode(&read_coin_request(1, 2).unwrap()).unwrap();
    assert_eq!(
        request,
        Frame {
            destination: 1,
            payload: vec![CMD_READ_COIN, 2],
        }
    );

    let slot0: u16 = 3;
    let slot1: u16 = (0x0A << 8) | 0x20;
    let frame = Frame {
        destination: 0,
        payload: vec![
            STATUS_OK,
            REPORT_OK,
            ((slot0 >> 8) & 0x3F) as u8,
            (slot0 & 0xFF) as u8,
            (((slot1 >> 8) & 0x3F) as u8) | (1 << 6),
            (slot1 & 0xFF) as u8,
        ],
    };
    let pairs = decode_coin_response(&frame, 2).unwrap();
    assert_eq!(pairs.len(), 2);

    use crate::gui::common::coin::{CoinCondition, CoinSlot};
    let decoded0 = CoinSlot::from_jvs(pairs[0].0, pairs[0].1);
    assert_eq!(decoded0.count, 3);
    assert_eq!(decoded0.condition, CoinCondition::Normal);
    let decoded1 = CoinSlot::from_jvs(pairs[1].0, pairs[1].1);
    assert_eq!(decoded1.count, slot1);
    assert_eq!(decoded1.condition, CoinCondition::Jam);
}

#[test]
fn coin_response_rejects_short_or_bad_status_payloads() {
    let short = Frame {
        destination: 0,
        payload: vec![STATUS_OK, REPORT_OK, 0x00],
    };
    assert!(decode_coin_response(&short, 2).is_err());
    let bad_status = Frame {
        destination: 0,
        payload: vec![0x02, REPORT_OK, 0x00, 0x00],
    };
    assert!(decode_coin_response(&bad_status, 1).is_err());
    let not_master = Frame {
        destination: 1,
        payload: vec![STATUS_OK, REPORT_OK, 0x00, 0x00],
    };
    assert!(decode_coin_response(&not_master, 1).is_err());
}

#[test]
fn phase_b_coin_decrease_and_write_gpio_frames_are_exact() {
    let dec = decode(&coin_decrease_request(1, 0, 1).unwrap()).unwrap();
    assert_eq!(dec.payload, vec![CMD_COIN_DECREASE, 0, 0x00, 0x01]);

    let gpio = decode(&write_gpio_request(1, &[0xFF, 0x00]).unwrap()).unwrap();
    assert_eq!(gpio.payload, vec![CMD_WRITE_GPIO, 2, 0xFF, 0x00]);
}

#[test]
fn all_jvs_ba_bits_match_the_verified_map() {
    let mapping = [
        (11, Button::Ba1),
        (10, Button::Ba2),
        (8, Button::Ba3),
        (7, Button::Ba4),
        (6, Button::Ba5),
        (5, Button::Ba6),
        (4, Button::Ba7),
        (3, Button::Ba8),
    ];
    for (bit, button) in mapping {
        let value = SWITCH_IDLE & !(1 << bit);
        let frame = Frame {
            destination: 0,
            payload: vec![1, 1, 0, (value >> 8) as u8, value as u8],
        };
        assert_eq!(
            decode_switch_response(&frame, 0).unwrap().bits(),
            button.mask()
        );
    }
}
