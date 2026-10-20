use super::*;

#[test]
fn fields_are_bounded_truncated_and_unknown_ones_ignored() {
    let long = "x".repeat(300);
    let Message::Progress(progress) = parse_line(
        format!(r#"{{"type":"progress","stage":"defragging","text":"{long}","eta":12}}"#)
            .as_bytes(),
    )
    .unwrap()
    .unwrap() else {
        panic!("progress expected");
    };
    assert_eq!(progress.stage, Stage::Other("defragging".into()));
    assert_eq!(progress.stage.as_str(), "defragging");
    assert_eq!(progress.text.len(), 256);
    assert_eq!(progress.percent, None);

    assert_eq!(
        parse_line(br#"{"type":"progress","stage":"patching","percent":100.5}"#),
        Err(Invalid::Field("percent"))
    );
    assert_eq!(
        parse_line(br#"{"type":"progress","stage":"Patching"}"#),
        Err(Invalid::Field("stage"))
    );
    assert_eq!(
        parse_line(br#"{"type":"progress","percent":5}"#),
        Err(Invalid::Field("progress"))
    );
    assert_eq!(
        parse_line(br#"{"type":"game_state","credits":9999}"#),
        Ok(Some(Message::GameState(GameState {
            credits: Some(9999),
            paid_play: None,
            free_play: false,
            mode: GameMode::Unknown,
        })))
    );
    assert_eq!(
        parse_line(br#"{"type":"game_state","credits":10000}"#),
        Err(Invalid::Field("credits"))
    );
    assert_eq!(
        parse_line(br#"{"type":"game_state","mode":"sleeping"}"#),
        Err(Invalid::Field("mode"))
    );
    assert!(!GameState::default().payment_known());
    assert!(GameState {
        paid_play: Some(false),
        ..GameState::default()
    }
    .payment_known());

    let Message::Log(log) = parse_line(
        format!(
            r#"{{"type":"log","level":"error","text":"{}"}}"#,
            "y".repeat(2_000)
        )
        .as_bytes(),
    )
    .unwrap()
    .unwrap() else {
        panic!("log expected");
    };
    assert_eq!((log.level, log.text.len()), (LogLevel::Error, 1_024));
    assert_eq!(parse_line(b""), Err(Invalid::NotJson));
    assert_eq!(parse_line(b"[]"), Err(Invalid::NotAnObject));
    assert_eq!(parse_line(b"{}"), Err(Invalid::MissingType));
    assert_eq!(parse_line(&[0xff, 0xfe]), Err(Invalid::NotUtf8));
    assert_eq!(parse_line(br#"{"type":"future"}"#), Ok(None));
}

#[test]
fn stage_words_are_lowercase_and_bounded() {
    assert_eq!(Stage::parse("backing_up"), Some(Stage::BackingUp));
    assert_eq!(Stage::parse("complete").unwrap().as_str(), "complete");
    assert_eq!(Stage::parse("running"), Some(Stage::Running));
    assert_eq!(Stage::Running.as_str(), "running");
    assert_eq!(Stage::parse(""), None);
    assert_eq!(Stage::parse(&"a".repeat(33)), None);
    assert_eq!(Stage::parse("has space"), None);
    assert_eq!(Stage::parse("UPPER"), None);
    assert_eq!(LogLevel::Debug.as_str(), "debug");
}
