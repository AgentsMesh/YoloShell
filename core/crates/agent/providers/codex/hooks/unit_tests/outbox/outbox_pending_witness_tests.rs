use super::outbox_receipt_test_support::ReceiptFixture;
use crate::agent::hooks::FIRST_SEQUENCE;
use std::panic::AssertUnwindSafe;

const NEXT_SEQUENCE_AFTER_FIRST: u64 = FIRST_SEQUENCE + FIRST_SEQUENCE;
const SINGLE_BINDING: usize = 1;

#[test]
fn enqueue_commit_atomically_persists_event_and_witness() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    let state = fixture.state();

    assert_eq!(state.pending.len(), SINGLE_BINDING);
    assert_eq!(state.pending_occurrence_witnesses.len(), SINGLE_BINDING);
    assert_eq!(state.pending.front(), Some(&persisted.assigned));
    assert_eq!(state.next_sequence, NEXT_SEQUENCE_AFTER_FIRST);
}

#[test]
fn failed_enqueue_commit_keeps_raw_and_pending_state_intact() {
    let fixture = ReceiptFixture::new();
    let raw = fixture.intake.persist_occurrence(fixture.raw()).unwrap();
    let occurrence = fixture.intake.find_exact_occurrence(&raw).unwrap().unwrap();
    let identity = occurrence.durable_identity().unwrap();
    let before = fixture.state();
    crate::codex_hooks::set_outbox_commit_hook(fixture.outbox.path().to_path_buf(), || {
        std::panic::panic_any(())
    });

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        fixture
            .outbox
            .enqueue_admitted(
                raw.clone(),
                Some(&identity),
                |_, _| Ok(()),
                |_, _, _| Ok(()),
            )
            .unwrap();
    }));

    assert!(result.is_err());
    assert_eq!(fixture.state(), before);
    assert!(
        fixture
            .intake
            .find_exact_occurrence(&raw)
            .unwrap()
            .is_some()
    );
}

#[test]
fn generic_replay_and_ack_cannot_bypass_a_pending_witness() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    let before = fixture.state();

    assert!(fixture.outbox.enqueue(persisted.raw.clone()).is_err());
    assert!(
        fixture
            .outbox
            .acknowledge(persisted.assigned.sequence.0, persisted.assigned.event_id)
            .is_err()
    );
    assert_eq!(fixture.state(), before);
}

#[test]
fn exact_pending_replay_rejects_same_content_vnode_replacement() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    assert_eq!(
        fixture
            .outbox
            .authoritative_intake_occurrence(&persisted.occurrence)
            .unwrap(),
        Some(persisted.assigned.clone())
    );

    let before = fixture.state();
    let replacement = fixture.replace_same_content(&persisted);
    assert!(
        fixture
            .outbox
            .authoritative_intake_occurrence(&replacement)
            .is_err()
    );
    assert_eq!(fixture.state(), before);
}
