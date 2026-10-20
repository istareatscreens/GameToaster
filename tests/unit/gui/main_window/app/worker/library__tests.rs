use super::*;

#[test]
fn presentation_cleanup_never_deletes_the_referenced_side_of_a_commit() {
    let old = Some(PathBuf::from("launcher_assets/game_images/old.png"));
    let new = Some(PathBuf::from("launcher_assets/game_images/new.png"));
    assert_eq!(
        presentation_cleanup_plan(true, &old, &new),
        old.clone().map(|path| (path, CleanupReferences::Committed))
    );
    assert_eq!(
        presentation_cleanup_plan(false, &old, &new),
        new.clone().map(|path| (path, CleanupReferences::Original))
    );
    assert_eq!(presentation_cleanup_plan(true, &old, &old), None);
}
