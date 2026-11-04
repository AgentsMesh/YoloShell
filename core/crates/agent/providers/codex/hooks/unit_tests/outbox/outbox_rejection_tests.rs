use super::super::constants::{FIRST_SEQUENCE, OUTBOX_FILE_NAME};
use super::{envelope, identity};
use crate::agent::hooks::{EventSequence, HookViolation, HookProviderKind};
use crate::codex_hooks::{CodexOutbox, CodexOutboxState};
use uuid::Uuid;

const SECOND_SEQUENCE: u64 = FIRST_SEQUENCE + FIRST_SEQUENCE;

#[test]
fn duplicate_rejection_markers_are_rejected_at_the_storage_boundary() {
    let (mut state, rejected) = rejected_state();
    let marker = marker(&rejected);
    state.durable_rejections.extend([marker, marker]);

    assert_rejected(state);
}

#[test]
fn a_second_rejection_marker_exhausts_the_authority_capacity() {
    let (mut state, rejected) = rejected_state();
    super::super::outbox_rejection::insert(&mut state, &rejected, HookViolation::ProviderMismatch)
        .unwrap();
    let mut second = rejected.clone();
    second.event_id = Uuid::new_v4();
    second.sequence = EventSequence(SECOND_SEQUENCE);

    let error = super::super::outbox_rejection::insert(
        &mut state,
        &second,
        HookViolation::ProviderMismatch,
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        super::super::constants::errors::OUTBOX_FULL
    );
}

#[test]
fn rejection_marker_must_be_the_last_retained_event() {
    let (mut state, rejected) = rejected_state();
    let owner = identity();
    let mut tail = envelope(&owner, Uuid::new_v4());
    tail.sequence = EventSequence(SECOND_SEQUENCE);
    state.pending.push_back(tail);
    state.next_sequence = SECOND_SEQUENCE.checked_add(FIRST_SEQUENCE).unwrap();
    state.durable_rejections.push_back(marker(&rejected));

    assert_rejected(state);
}

#[test]
fn orphan_rejection_markers_are_rejected_at_the_storage_boundary() {
    let (state, rejected) = rejected_state();
    let mut orphaned = CodexOutboxState {
        owner: state.owner,
        ..CodexOutboxState::default()
    };
    orphaned.durable_rejections.push_back(marker(&rejected));

    assert_rejected(orphaned);
}

fn rejected_state() -> (CodexOutboxState, crate::agent::hooks::HookEnvelope) {
    let owner = identity();
    let mut rejected = envelope(&owner, Uuid::new_v4());
    rejected.sequence = EventSequence::FIRST;
    rejected.provider = HookProviderKind::Claude;
    let mut state = CodexOutboxState {
        next_sequence: SECOND_SEQUENCE,
        owner: Some(owner.hook),
        ..CodexOutboxState::default()
    };
    state.pending.push_back(rejected.clone());
    (state, rejected)
}

fn marker(event: &crate::agent::hooks::HookEnvelope) -> super::super::CodexDurableRejection {
    super::super::CodexDurableRejection::capture(event, HookViolation::ProviderMismatch).unwrap()
}

fn assert_rejected(state: CodexOutboxState) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTBOX_FILE_NAME);
    crate::paths::atomic_write_json(&path, &state).unwrap();
    assert!(CodexOutbox::new(path).is_err());
}
