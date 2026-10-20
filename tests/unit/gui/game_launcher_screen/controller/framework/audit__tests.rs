use super::*;

#[test]
fn the_proof_mirrors_the_live_counts() {
    let audit = ResourceAudit::new();
    assert_eq!(audit.counts(), (0, 0));
    assert_eq!(audit.proof(), ReleaseProof::default());
    audit.handle_opened();
    audit.handle_opened();
    audit.reader_started();
    assert_eq!(audit.counts(), (2, 1));
    assert_eq!(
        audit.proof(),
        ReleaseProof {
            open_handles: 2,
            live_readers: 1,
        }
    );
    audit.handle_closed();
    audit.reader_stopped();
    audit.handle_closed();
    assert_eq!(audit.proof(), ReleaseProof::default());
}

#[test]
fn audits_are_independent_of_each_other() {
    let first = ResourceAudit::new();
    let second = ResourceAudit::new();
    first.handle_opened();
    second.reader_started();
    assert_eq!(first.counts(), (1, 0));
    assert_eq!(second.counts(), (0, 1));
}
