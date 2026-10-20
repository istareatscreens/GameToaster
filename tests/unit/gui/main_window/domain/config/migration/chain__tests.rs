use super::super::super::parse_document;
use super::super::fixtures::{self, as_lf, as_written};
use super::*;

#[test]
fn every_earlier_version_has_exactly_one_step() {
    for version in 1..SCHEMA_VERSION {
        assert_eq!(
            STEPS.iter().filter(|step| step.from == version).count(),
            1,
            "version {version}"
        );
    }
    assert!(STEPS
        .iter()
        .all(|step| (1..SCHEMA_VERSION).contains(&step.from)));
}

#[test]
fn a_version_no_step_starts_from_is_not_upgraded() {
    let error = upgrade(parse_document("").unwrap(), 0)
        .err()
        .expect("no step starts from 0");
    assert_eq!(error, "no step upgrades version 0");
}

#[test]
fn the_upgraded_document_is_read_by_the_current_rules() {
    let text = as_written(fixtures::V2_IO4)
        .replace("auto_hide_on_launch=true", "auto_hide_on_launch=perhaps");
    assert_ne!(text, as_written(fixtures::V2_IO4));
    let error = upgrade(parse_document(&text).unwrap(), 2)
        .err()
        .expect("the current rules refuse the value");
    assert!(error.contains("auto_hide_on_launch"), "{error}");
}

#[test]
fn the_steps_findings_reach_the_result() {
    let upgraded = upgrade(parse_document(&as_lf(fixtures::V1_FULL)).unwrap(), 1).unwrap();
    assert_eq!(upgraded.plugin_needed, ["日本語 ゲーム", "Second Prepared"]);
}
