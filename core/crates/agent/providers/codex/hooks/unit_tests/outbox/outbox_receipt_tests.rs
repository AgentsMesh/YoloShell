use super::outbox_receipt_test_support::ReceiptFixture;
use crate::agent::hooks::{EventSequence, MAX_REPLAY_WINDOW};
use std::panic::AssertUnwindSafe;

const ACK_HISTORY_BOUNDARY_TAIL: usize = MAX_REPLAY_WINDOW - 1;
const SINGLE_RECEIPT: usize = 1;

#[test]
fn acknowledgement_and_exact_receipt_commit_atomically() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();

    let state = fixture.state();
    assert!(state.pending.is_empty());
    assert_eq!(state.acknowledged_sequence, persisted.assigned.sequence.0);
    assert_eq!(state.acknowledged_events.back(), Some(&persisted.assigned));
    assert_eq!(state.occurrence_receipts.len(), SINGLE_RECEIPT);
}

#[test]
fn failed_ack_commit_preserves_the_complete_pre_ack_state() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    let before = fixture.state();
    crate::codex_hooks::set_outbox_commit_hook(fixture.outbox.path().to_path_buf(), || {
        std::panic::panic_any(())
    });

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        fixture.acknowledge(&persisted).unwrap()
    }));

    assert!(result.is_err());
    assert_eq!(fixture.state(), before);
    assert!(
        fixture
            .intake
            .find_exact_occurrence(&persisted.raw)
            .unwrap()
            .is_some()
    );
}

#[test]
fn receipt_replay_survives_the_acknowledged_history_boundary() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();
    fixture.append_acknowledged(ACK_HISTORY_BOUNDARY_TAIL);
    assert_eq!(
        fixture.state().acknowledged_events.front(),
        Some(&persisted.assigned)
    );
    assert_eq!(
        fixture
            .outbox
            .authoritative_intake_occurrence(&persisted.occurrence)
            .unwrap(),
        Some(persisted.assigned.clone())
    );

    fixture.append_acknowledged(SINGLE_RECEIPT);
    assert_ne!(
        fixture.state().acknowledged_events.front(),
        Some(&persisted.assigned)
    );
    assert_eq!(
        fixture
            .outbox
            .authoritative_intake_occurrence(&persisted.occurrence)
            .unwrap(),
        Some(persisted.assigned)
    );
}

#[test]
fn changed_payload_sequence_and_sequence_collision_fail_closed() {
    let fixture = ReceiptFixture::new();
    let first = fixture.persist();
    fixture.acknowledge(&first).unwrap();
    let second = fixture.persist();
    fixture.acknowledge(&second).unwrap();
    let before = fixture.state();

    let mut changed = first.raw.clone();
    changed.provider_session_id.push_str("-changed");
    assert!(
        fixture
            .outbox
            .enqueue_admitted(
                changed,
                Some(&first.identity),
                |_, admission| Ok(admission),
                |_, _, _| Ok(())
            )
            .is_err()
    );
    let mut wrong_sequence = first.assigned.clone();
    wrong_sequence.sequence = EventSequence(first.assigned.sequence.0 + 1);
    assert!(
        before
            .proves_occurrence_receipt(&wrong_sequence, &first.identity)
            .is_err()
    );
    let mut collision = first.assigned;
    collision.sequence = second.assigned.sequence;
    assert!(
        before
            .proves_occurrence_receipt(&collision, &first.identity)
            .is_err()
    );
    assert_eq!(fixture.state(), before);
}

#[test]
fn same_payload_vnode_replacement_cannot_consume_a_receipt() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();
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

#[test]
fn exact_receipt_removal_is_idempotent() {
    let fixture = ReceiptFixture::new();
    let persisted = fixture.persist();
    fixture.acknowledge(&persisted).unwrap();
    fixture.intake.complete(&persisted.occurrence).unwrap();

    fixture
        .outbox
        .remove_occurrence_receipt(&persisted.assigned, &persisted.raw, &persisted.identity)
        .unwrap();
    assert_eq!(fixture.receipt_count(), usize::default());
    fixture
        .outbox
        .remove_occurrence_receipt(&persisted.assigned, &persisted.raw, &persisted.identity)
        .unwrap();
    assert_eq!(fixture.receipt_count(), usize::default());
}
