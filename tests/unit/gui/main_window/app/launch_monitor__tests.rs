use super::*;
use crate::gui::common::harness::locale;

const ABSENT: &str = "gametoaster-test-absent-7c1e.exe";

fn never_started() -> String {
    let mut pending = PendingHide::new(GameId(1), ABSENT.into(), None);
    pending.appear_deadline = Instant::now();
    match pending.tick() {
        HideTick::Failed(explanation) => explanation,
        _ => panic!("a game that never appears fails its launch"),
    }
}

#[test]
fn a_failed_launch_reads_in_japanese_with_the_technical_detail_after_it() {
    let _locale = locale("ja");
    assert_eq!(
        t!("errors.game_did_not_start"),
        "ゲームが起動しませんでした"
    );
    assert_eq!(
        never_started(),
        format!(
            "ゲームプロセス「{ABSENT}」が時間内に起動しませんでした。ゲーム画面を確認できるよう、ランチャーは開いたままです。"
        )
    );

    assert_eq!(
        t!("errors.launch_failed", game => "Game"),
        "Game を起動できませんでした"
    );
    assert_eq!(
        t!(
            "plugins.error.prepare_after_deploy_failed",
            error => "The system cannot find the file specified. (os error 2)"
        ),
        "配置はコミットされましたが、プラグインの準備処理が失敗しました：The system cannot find the file specified. (os error 2)"
    );
}

#[test]
fn a_failed_launch_reads_in_english_exactly_as_before() {
    let _locale = locale("en");
    assert_eq!(
        never_started(),
        format!(
            "The game process “{ABSENT}” did not start in time. The launcher stayed open so you can check the game window."
        )
    );
}

#[test]
fn a_failed_wait_is_reported_once_and_keeps_the_launch_in_flight_with_its_reason() {
    let _locale = locale("en");
    let mut pending = PendingHide::new(GameId(1), ABSENT.into(), None);
    assert!(!pending.has_failed());
    assert_eq!(pending.status(), t!("status.loading", exe => ABSENT));
    pending.appear_deadline = Instant::now();

    let HideTick::Failed(reason) = pending.tick() else {
        panic!("the first look past the deadline reports the failure");
    };
    assert!(pending.has_failed());
    assert_eq!(
        pending.status(),
        reason,
        "the status line carries the failure"
    );

    pending.next_look = Instant::now();
    assert!(matches!(pending.tick(), HideTick::Wait));
    assert!(matches!(pending.tick(), HideTick::Wait));
    assert_eq!(pending.status(), reason);
}

#[test]
fn only_the_dialog_that_reported_the_failure_counts_as_its_report() {
    let _locale = locale("en");
    let title = t!("errors.game_did_not_start").to_string();
    let mut pending = PendingHide::new(GameId(1), ABSENT.into(), None);
    assert!(
        !pending.reported_by(Some(&Dialog::error(title.clone(), pending.status()))),
        "nothing has failed yet"
    );
    pending.appear_deadline = Instant::now();
    let HideTick::Failed(reason) = pending.tick() else {
        panic!("the first look past the deadline reports the failure");
    };

    assert!(pending.reported_by(Some(&Dialog::error(title.clone(), reason.clone()))));
    assert!(!pending.reported_by(Some(&Dialog::error(title.clone(), "Another failure"))));
    assert!(!pending.reported_by(Some(&Dialog::notice(title, reason))));
    assert!(!pending.reported_by(None));
}

#[test]
fn a_program_that_appears_after_the_failure_still_completes_the_launch() {
    let me = std::env::current_exe().unwrap();
    let name = me.file_name().unwrap().to_string_lossy().into_owned();
    let mut pending = PendingHide::new(GameId(1), name, Some(me));
    pending.failed = Some("did not start in time".into());
    pending.next_look = Instant::now();

    assert!(
        matches!(pending.tick(), HideTick::Wait),
        "found: now confirming"
    );
    let Some((_, _, render_deadline)) = pending.confirming.as_mut() else {
        panic!("the late process is being confirmed");
    };
    *render_deadline = Instant::now();
    assert!(matches!(pending.tick(), HideTick::Confirmed));
}

#[test]
fn a_plugin_launch_is_confirmed_only_once_its_program_shows_a_window() {
    let me = std::env::current_exe().unwrap();
    let name = me.file_name().unwrap().to_string_lossy().into_owned();
    let mut pending = PendingHide::new(GameId(1), name, Some(me)).for_plugin();
    assert!(pending.follows_after_confirmation());
    assert!(
        matches!(pending.tick(), HideTick::Wait),
        "found: now confirming"
    );
    let Some((_, _, render_deadline)) = pending.confirming.as_mut() else {
        panic!("the program is being confirmed");
    };
    *render_deadline = Instant::now();
    for _ in 0..3 {
        assert!(matches!(pending.tick(), HideTick::Wait));
    }
    assert!(!pending.has_failed());
}

#[test]
fn a_confirmed_program_gone_long_enough_is_reported_once_and_its_return_too() {
    let start = Instant::now();
    let at = |seconds: u64| start + Duration::from_secs(seconds);
    let mut tracker = GoneTracker::new(GONE_AFTER);

    assert_eq!(tracker.observe(false, at(0)), None);
    assert_eq!(tracker.observe(false, at(100)), None);
    assert_eq!(tracker.observe(true, at(101)), None);
    assert_eq!(tracker.observe(false, at(102)), None);
    assert_eq!(tracker.observe(false, at(131)), None, "gone 29 seconds");
    assert_eq!(tracker.observe(false, at(132)), Some(Presence::Gone));
    assert_eq!(tracker.observe(false, at(200)), None, "reported once");
    assert_eq!(tracker.observe(true, at(201)), Some(Presence::Back));
    assert_eq!(tracker.observe(true, at(202)), None);

    assert_eq!(tracker.observe(false, at(203)), None);
    assert_eq!(tracker.observe(true, at(210)), None);
    assert_eq!(tracker.observe(false, at(211)), None);
    assert_eq!(tracker.observe(false, at(241)), Some(Presence::Gone));
}

#[test]
fn the_game_no_longer_running_dialog_reads_in_every_language_and_is_recognised() {
    for (language, title) in [
        ("en", "The game is no longer running"),
        ("ja", "ゲームが実行されていません"),
        ("zh-Hans", "游戏已不再运行"),
        ("zh-Hant", "遊戲已不再執行"),
    ] {
        let _locale = locale(language);
        assert_eq!(t!("errors.game_no_longer_running"), title, "{language}");
        let detail = t!("errors.game_no_longer_running_detail", exe => "game.exe").to_string();
        assert!(detail.contains("game.exe"), "{language}: {detail}");
        assert!(detail.contains("30"), "{language}: {detail}");
        let dialog = Dialog::error(t!("errors.game_no_longer_running"), detail.clone());
        assert!(reports_gone(Some(&dialog)), "{language}");
        assert!(!reports_gone(Some(&Dialog::error(
            t!("errors.game_did_not_start"),
            detail
        ))));
        assert!(!reports_gone(None));
    }
}
