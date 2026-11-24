use super::outbox_receipt_test_support::ReceiptFixture;

const NO_RECEIPTS_REMOVED: usize = 0;
const ONE_RECEIPT_REMOVED: usize = 1;
const ONE_BINDING: usize = 1;
const FENCE_REJECTION: &str = "fence rejected";

#[test]
fn sweep_keeps_a_receipt_with_the_exact_raw_artifact() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();

    assert_eq!(
        fixture
            .outbox
            .sweep_occurrence_receipts(&fixture.intake, || Ok(()))
            .unwrap(),
        NO_RECEIPTS_REMOVED
    );
    assert_eq!(fixture.receipt_count(), ONE_BINDING);
}

#[test]
fn sweep_removes_a_receipt_when_the_canonical_artifact_is_missing() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();
    fixture.intake.complete(&persisted.occurrence).unwrap();

    assert_eq!(
        fixture
            .outbox
            .sweep_occurrence_receipts(&fixture.intake, || Ok(()))
            .unwrap(),
        ONE_RECEIPT_REMOVED
    );
    assert_eq!(fixture.receipt_count(), NO_RECEIPTS_REMOVED);
}

#[test]
fn sweep_defers_a_missing_receipt_when_parent_sync_fails() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();
    fixture.intake.complete(&persisted.occurrence).unwrap();
    crate::paths::test_support::fail_next_directory_sync();

    assert!(
        fixture
            .outbox
            .sweep_occurrence_receipts(&fixture.intake, || Ok(()))
            .is_err()
    );
    assert_eq!(fixture.receipt_count(), ONE_BINDING);
    assert_eq!(
        fixture
            .outbox
            .sweep_occurrence_receipts(&fixture.intake, || Ok(()))
            .unwrap(),
        ONE_RECEIPT_REMOVED
    );
    assert_eq!(fixture.receipt_count(), NO_RECEIPTS_REMOVED);
}

#[test]
fn sweep_rejects_a_same_name_same_content_vnode_replacement() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();
    fixture.replace_same_content(&persisted);
    let before = fixture.state();

    assert!(
        fixture
            .outbox
            .sweep_occurrence_receipts(&fixture.intake, || Ok(()))
            .is_err()
    );
    assert_eq!(fixture.state(), before);
}

#[test]
fn sweep_never_removes_a_pending_witness() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();

    assert_eq!(
        fixture
            .outbox
            .sweep_occurrence_receipts(&fixture.intake, || Ok(()))
            .unwrap(),
        NO_RECEIPTS_REMOVED
    );
    let state = fixture.state();
    assert_eq!(state.pending_occurrence_witnesses.len(), ONE_BINDING);
    assert_eq!(state.pending.front(), Some(&persisted.assigned));
}

#[test]
fn failed_writer_fence_keeps_all_receipts() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();
    let before = fixture.state();

    assert!(
        fixture
            .outbox
            .sweep_occurrence_receipts(&fixture.intake, || {
                Err(anyhow::anyhow!(FENCE_REJECTION))
            })
            .is_err()
    );
    assert_eq!(fixture.state(), before);
}
