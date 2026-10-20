use super::*;

#[test]
fn gate_disables_verbose_events_but_crash_is_always_written() {
    let dir = std::env::temp_dir().join(format!("gametoaster-log-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("GameToaster.log");

    let log = LauncherLog::open(&dir).unwrap();

    log.event("verbose", &[("k", "v".into())]);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
    log.crash("panic", &[("location", "x:1:1".into())]);
    let after_crash = std::fs::read_to_string(&path).unwrap();
    assert!(after_crash.contains("operation=panic"));
    assert!(!after_crash.contains("operation=verbose"));

    let clone = log.clone();
    log.set_enabled(true);
    clone.event("verbose", &[("k", "v".into())]);
    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .contains("operation=verbose"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn backend_log_sink_accepts_only_backend_warnings_and_errors() {
    use log::Level::{Debug, Error, Info, Warn};
    assert!(BackendLogSink::accepts("eframe", Error));
    assert!(BackendLogSink::accepts(
        "eframe::native::glow_integration",
        Error
    ));
    assert!(BackendLogSink::accepts(
        "winit::platform_impl::platform::monitor",
        Warn
    ));
    assert!(BackendLogSink::accepts("glutin::api::wgl::surface", Error));
    assert!(BackendLogSink::accepts("egui_winit", Warn));
    assert!(!BackendLogSink::accepts("eframe", Info));
    assert!(!BackendLogSink::accepts("eframe", Debug));
    assert!(!BackendLogSink::accepts("eframe_extras", Error));
    assert!(!BackendLogSink::accepts("GameToaster::plugin", Error));
}

#[test]
fn backend_log_sink_writes_to_the_crash_channel_with_diagnostics_off() {
    let dir = std::env::temp_dir().join(format!("gametoaster-log-sink-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("GameToaster.log");
    let log = LauncherLog::open(&dir).unwrap();
    let sink = BackendLogSink::new(log);

    log::Log::log(
        &sink,
        &log::Record::builder()
            .level(log::Level::Error)
            .target("eframe::native::glow_integration")
            .args(format_args!(
                "Failed to initialize a window for immediate viewport"
            ))
            .build(),
    );
    log::Log::log(
        &sink,
        &log::Record::builder()
            .level(log::Level::Info)
            .target("eframe")
            .args(format_args!("chatter"))
            .build(),
    );
    log::Log::log(
        &sink,
        &log::Record::builder()
            .level(log::Level::Error)
            .target("other_crate")
            .args(format_args!("not ours"))
            .build(),
    );

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("operation=backend-log"), "{text}");
    assert!(text.contains("level=\"ERROR\""), "{text}");
    assert!(
        text.contains("target=\"eframe::native::glow_integration\""),
        "{text}"
    );
    assert!(text.contains("Failed to initialize a window"), "{text}");
    assert!(!text.contains("chatter"), "{text}");
    assert!(!text.contains("not ours"), "{text}");
    assert_eq!(text.lines().count(), 1, "{text}");

    std::fs::remove_dir_all(&dir).unwrap();
}
