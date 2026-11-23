use super::outbox_budget::{
    MAX_OCCURRENCE_BINDING_BYTES, MAX_OCCURRENCE_BINDINGS, NON_ENVELOPE_HEADROOM_BYTES,
    OCCURRENCE_BINDING_BUDGET_BYTES, STRUCTURAL_HEADROOM_BYTES,
};
use super::outbox_receipt_test_support::ReceiptFixture;
use crate::agent::hooks::MAX_REPLAY_WINDOW;
use crate::codex_hooks::{CodexOccurrenceBindingCapacity, MAX_OUTBOX_BYTES};
use std::sync::{Arc, Barrier};

const CAPACITY_FIXTURE_WORKERS: usize = 3;
const EXACT_REPLAY_WORKERS: usize = 1;
const LOCK_ISOLATION_RECEIPTS: usize = 1;
const RETAINED_BEFORE_LIMIT: usize = MAX_REPLAY_WINDOW - 1;
const ENVELOPE_RESERVATION_BYTES: usize = MAX_OUTBOX_BYTES - NON_ENVELOPE_HEADROOM_BYTES;

#[test]
fn capacity_allows_the_last_binding() {
    let fixture = ReceiptFixture::new();
    fixture.seed_receipts(RETAINED_BEFORE_LIMIT);
    let candidate = fixture.persist();

    assert_eq!(
        fixture
            .outbox
            .occurrence_binding_capacity(&candidate.raw, &candidate.identity)
            .unwrap(),
        CodexOccurrenceBindingCapacity::Available
    );
    fixture.acknowledge(&candidate).unwrap();
    assert_eq!(fixture.receipt_count(), MAX_OCCURRENCE_BINDINGS);
}

#[test]
fn full_bindings_request_recovery_without_mutation() {
    let fixture = ReceiptFixture::new();
    fixture.seed_receipts(MAX_OCCURRENCE_BINDINGS);
    let candidate = fixture.persist_unassigned();
    let identity = candidate.durable_identity().unwrap();
    let before = fixture.state();

    assert_eq!(
        fixture
            .outbox
            .occurrence_binding_capacity(candidate.envelope(), &identity)
            .unwrap(),
        CodexOccurrenceBindingCapacity::RecoveryRequired
    );
    assert!(
        fixture
            .outbox
            .ensure_occurrence_binding_capacity(candidate.envelope(), &identity)
            .is_err()
    );
    assert_eq!(fixture.state(), before);

    let removed = fixture
        .outbox
        .sweep_occurrence_receipts(&fixture.intake, || Ok(()))
        .unwrap();
    assert_eq!(removed, MAX_OCCURRENCE_BINDINGS);
    assert_eq!(
        fixture
            .outbox
            .occurrence_binding_capacity(candidate.envelope(), &identity)
            .unwrap(),
        CodexOccurrenceBindingCapacity::Available
    );
}

#[test]
fn occurrence_budget_accounts_for_both_envelope_windows() {
    assert_eq!(
        MAX_OUTBOX_BYTES,
        ENVELOPE_RESERVATION_BYTES + NON_ENVELOPE_HEADROOM_BYTES
    );
    assert_eq!(
        NON_ENVELOPE_HEADROOM_BYTES,
        OCCURRENCE_BINDING_BUDGET_BYTES + STRUCTURAL_HEADROOM_BYTES
    );
    assert_eq!(
        OCCURRENCE_BINDING_BUDGET_BYTES,
        MAX_OCCURRENCE_BINDINGS * MAX_OCCURRENCE_BINDING_BYTES
    );
}

#[test]
fn retained_binding_encoding_stays_within_its_allocation() {
    let fixture = ReceiptFixture::new();
    fixture.seed_receipts(RETAINED_BEFORE_LIMIT);
    let state = fixture.state();

    for witness in &state.pending_occurrence_witnesses {
        assert!(serde_json::to_vec(witness).unwrap().len() <= MAX_OCCURRENCE_BINDING_BYTES);
    }
    for receipt in &state.occurrence_receipts {
        assert!(serde_json::to_vec(receipt).unwrap().len() <= MAX_OCCURRENCE_BINDING_BYTES);
    }
}

#[test]
fn capacity_seeding_and_exact_replay_use_isolated_writer_locks() {
    let participants = CAPACITY_FIXTURE_WORKERS + EXACT_REPLAY_WORKERS;
    let barrier = Arc::new(Barrier::new(participants));
    let workers = (0..CAPACITY_FIXTURE_WORKERS)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let fixture = ReceiptFixture::new();
                barrier.wait();
                fixture.seed_receipts(LOCK_ISOLATION_RECEIPTS);
                assert_eq!(fixture.receipt_count(), LOCK_ISOLATION_RECEIPTS);
            })
        })
        .collect::<Vec<_>>();
    let exact = ReceiptFixture::new();
    barrier.wait();
    let persisted = exact.persist();
    let replacement = exact.replace_same_content(&persisted);

    assert!(
        exact
            .outbox
            .authoritative_intake_occurrence(&replacement)
            .is_err()
    );
    for worker in workers {
        worker.join().unwrap();
    }
}
