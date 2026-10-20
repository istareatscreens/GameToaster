use super::*;

fn one_slot(count: u16) -> Option<CoinReading> {
    Some(CoinReading {
        slots: vec![CoinSlot {
            count,
            condition: CoinCondition::Normal,
        }],
    })
}

#[test]
fn jvs_slot_decode_splits_count_and_condition() {
    let slot = CoinSlot::from_jvs(0x80 | 0x0A, 0x34);
    assert_eq!(slot.count, 0x0A34);
    assert_eq!(slot.condition, CoinCondition::Disconnected);

    let zero = CoinSlot::from_jvs(0x00, 0x00);
    assert_eq!(zero.count, 0);
    assert_eq!(zero.condition, CoinCondition::Normal);
}

#[test]
fn first_poll_is_baseline_only() {
    let mut monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![one_slot(5)])));
    assert_eq!(monitor.poll_new_coins(), 0);
}

#[test]
fn a_single_inserted_coin_is_observed_once() {
    let mut monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![
        one_slot(0),
        one_slot(1),
        one_slot(1),
    ])));
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 1);
    assert_eq!(monitor.poll_new_coins(), 0);
}

#[test]
fn multiple_coins_between_polls() {
    let mut monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![
        one_slot(10),
        one_slot(13),
    ])));
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 3);
}

#[test]
fn counter_wrap_is_handled() {
    let mut monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![
        one_slot(16383),
        one_slot(1),
    ])));
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 2);
}

#[test]
fn unavailable_tick_reports_zero_and_keeps_baseline() {
    let mut monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![
        one_slot(4),
        None,
        one_slot(6),
    ])));
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 2);
}

#[test]
fn multi_slot_coins_sum_across_slots() {
    let reading = |a: u16, b: u16| {
        Some(CoinReading {
            slots: vec![
                CoinSlot {
                    count: a,
                    condition: CoinCondition::Normal,
                },
                CoinSlot {
                    count: b,
                    condition: CoinCondition::Normal,
                },
            ],
        })
    };
    let mut monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![
        reading(0, 0),
        reading(2, 3),
    ])));
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 5);
}

#[test]
fn slot_count_change_resets_baseline_without_overcounting() {
    let one = one_slot(9);
    let two = Some(CoinReading {
        slots: vec![
            CoinSlot {
                count: 9,
                condition: CoinCondition::Normal,
            },
            CoinSlot {
                count: 4,
                condition: CoinCondition::Normal,
            },
        ],
    });
    let mut monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![one, two])));
    assert_eq!(monitor.poll_new_coins(), 0);
    assert_eq!(monitor.poll_new_coins(), 0);
}

#[test]
fn acceptor_gating_is_forwarded_and_reports_capability() {
    let mut gating = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![one_slot(0)])));
    assert!(gating.set_accept(false));
    let mut no_gate = CoinMonitor::new(Box::new(
        SimulatedCoinProvider::new(vec![one_slot(0)]).without_gating(),
    ));
    assert!(!no_gate.set_accept(false));
}

#[test]
fn provider_name_is_exposed() {
    let monitor = CoinMonitor::new(Box::new(SimulatedCoinProvider::new(vec![])));
    assert_eq!(monitor.provider_name(), "sim");
}
