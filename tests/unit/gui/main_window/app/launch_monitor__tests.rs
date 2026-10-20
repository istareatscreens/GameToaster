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
