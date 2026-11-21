use std::cell::Cell;
use std::collections::BTreeSet;

use super::*;

const SELECTED_A: &str = "aa";
const SELECTED_B: &str = "bb";
const OLDER_A: &str = "a0";
const OLDER_B: &str = "b0";

fn known() -> BTreeSet<String> {
    [SELECTED_A, SELECTED_B, OLDER_A, OLDER_B]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_selected_versions_files_are_selected_and_the_catalog_is_not_read() {
    let asked = Cell::new(0);
    let state = classify(
        [
            (Some(SELECTED_A), SELECTED_A),
            (Some(SELECTED_B), SELECTED_B),
        ],
        || {
            asked.set(asked.get() + 1);
            known()
        },
    );
    assert_eq!(state, FolderPatch::Selected);
    assert_eq!(asked.get(), 0, "a current folder reads no other version");
}

#[test]
fn a_missing_file_does_not_count_against_the_selected_version() {
    let asked = Cell::new(0);
    let state = classify([(None, SELECTED_A), (Some(SELECTED_B), SELECTED_B)], || {
        asked.set(asked.get() + 1);
        known()
    });
    assert_eq!(state, FolderPatch::Selected);
    assert_eq!(asked.get(), 0);
    assert_eq!(classify([(None, SELECTED_A)], known), FolderPatch::Selected);
}

#[test]
fn an_older_added_versions_files_are_an_added_version() {
    let asked = Cell::new(0);
    let state = classify(
        [(Some(OLDER_A), SELECTED_A), (Some(OLDER_B), SELECTED_B)],
        || {
            asked.set(asked.get() + 1);
            known()
        },
    );
    assert_eq!(state, FolderPatch::AddedVersion);
    assert_eq!(asked.get(), 1, "the catalog is read once");
}

#[test]
fn files_of_two_added_versions_mixed_are_an_added_version() {
    assert_eq!(
        classify(
            [(Some(OLDER_A), SELECTED_A), (Some(SELECTED_B), SELECTED_B)],
            known
        ),
        FolderPatch::AddedVersion
    );
    assert_eq!(
        classify([(Some(OLDER_A), SELECTED_A), (None, SELECTED_B)], known),
        FolderPatch::AddedVersion
    );
}

#[test]
fn a_file_of_no_added_version_is_unknown() {
    assert_eq!(
        classify(
            [(Some("ff"), SELECTED_A), (Some(SELECTED_B), SELECTED_B)],
            known
        ),
        FolderPatch::Unknown
    );
}

#[test]
fn one_unknown_file_among_older_ones_makes_the_whole_patch_unknown() {
    assert_eq!(
        classify(
            [(Some(OLDER_A), SELECTED_A), (Some("ff"), SELECTED_B)],
            known
        ),
        FolderPatch::Unknown
    );
    assert_eq!(
        classify(
            [
                (Some("ff"), SELECTED_A),
                (Some(OLDER_B), SELECTED_B),
                (None, "cc")
            ],
            known
        ),
        FolderPatch::Unknown
    );
}

#[test]
fn no_known_hashes_make_every_differing_file_unknown() {
    assert_eq!(
        classify([(Some(OLDER_A), SELECTED_A)], BTreeSet::new),
        FolderPatch::Unknown
    );
}

#[test]
fn hashes_are_compared_without_regard_to_case() {
    assert_eq!(
        classify([(Some("AA"), SELECTED_A)], known),
        FolderPatch::Selected
    );
    assert_eq!(
        classify([(Some("A0"), SELECTED_A)], known),
        FolderPatch::AddedVersion
    );
}

#[test]
fn no_plugin_files_are_selected() {
    let none: [(Option<&str>, &str); 0] = [];
    assert_eq!(classify(none, known), FolderPatch::Selected);
}
