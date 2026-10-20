use super::*;

#[test]
fn diagnostics_redact_paths_and_rate_limit_repeats() {
    let redacted = redact_device_path(r"\\?\hid#vid_0ca3&pid_0021#serial");
    assert!(redacted.starts_with("device:"));
    assert!(!redacted.contains("serial"));

    let start = Instant::now();
    let mut limiter = RateLimiter::new(Duration::from_secs(1));
    assert_eq!(limiter.admit("read-error", start), Some(0));
    assert_eq!(
        limiter.admit("read-error", start + Duration::from_millis(10)),
        None
    );
    assert_eq!(
        limiter.admit("read-error", start + Duration::from_secs(2)),
        Some(1)
    );
}
