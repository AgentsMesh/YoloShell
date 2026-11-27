use super::super::constants::OUTBOX_LOCK_SUFFIX;
use super::{envelope, identity};
use crate::agent::hooks::CURRENT_SCHEMA_VERSION;
use crate::codex_hooks::{CodexOutbox, CodexOutboxState};
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;

const SCHEMA_VERSION_INCREMENT: u16 = 1;
const EXPECTED_DRAINED_SEQUENCE: u64 = 2;
const EXPECTED_NEXT_SEQUENCE: u64 = 3;
const CHANGED_CWD: &str = "/workspace/changed";
const NON_NORMAL_CWD: &str = "/workspace/../outside";

#[test]
fn allocates_and_replays_idempotently() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let identity = identity();
    let first = outbox.enqueue(envelope(&identity, Uuid::new_v4())).unwrap();
    assert_eq!(first.sequence.0, 1);
    let replay = outbox.enqueue(envelope(&identity, first.event_id)).unwrap();
    assert_eq!(replay, first);
    assert_eq!(outbox.pending().unwrap(), vec![first.clone()]);
    outbox
        .acknowledge(first.sequence.0, first.event_id)
        .unwrap();
    assert!(outbox.pending().unwrap().is_empty());
}

#[test]
fn rejects_acknowledgement_gap() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let first = outbox
        .enqueue(envelope(&identity(), Uuid::new_v4()))
        .unwrap();
    assert!(
        outbox
            .acknowledge(first.sequence.0 + 1, Uuid::new_v4())
            .is_err()
    );
}

#[test]
fn acknowledged_replay_keeps_original_sequence() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let identity = identity();
    let first = outbox.enqueue(envelope(&identity, Uuid::new_v4())).unwrap();
    outbox
        .acknowledge(first.sequence.0, first.event_id)
        .unwrap();
    let replay = outbox.enqueue(envelope(&identity, first.event_id)).unwrap();
    assert_eq!(replay.sequence, first.sequence);
    assert!(outbox.pending().unwrap().is_empty());
}

#[test]
fn acknowledged_replay_with_changed_payload_is_rejected() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let identity = identity();
    let first = outbox.enqueue(envelope(&identity, Uuid::new_v4())).unwrap();
    outbox
        .acknowledge(first.sequence.0, first.event_id)
        .unwrap();
    let mut changed = envelope(&identity, first.event_id);
    changed.provider_session_id = "different-session".to_owned();
    assert!(outbox.enqueue(changed).is_err());
}

#[test]
fn duplicate_event_with_a_changed_schema_is_rejected() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let identity = identity();
    let event_id = Uuid::new_v4();
    outbox.enqueue(envelope(&identity, event_id)).unwrap();
    let mut changed = envelope(&identity, event_id);
    changed.schema_version = CURRENT_SCHEMA_VERSION.saturating_add(SCHEMA_VERSION_INCREMENT);
    assert!(outbox.enqueue(changed).is_err());
}

#[test]
fn duplicate_event_with_changed_metadata_is_rejected() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let identity = identity();
    let event_id = Uuid::new_v4();
    outbox.enqueue(envelope(&identity, event_id)).unwrap();
    let mut changed = envelope(&identity, event_id);
    changed.metadata.cwd = Some(PathBuf::from(CHANGED_CWD));
    assert!(outbox.enqueue(changed).is_err());
}

#[test]
fn bound_outbox_rejects_a_different_invocation() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("events.json");
    let first_identity = identity();
    let first = CodexOutbox::for_identity(&path, &first_identity.hook).unwrap();
    first
        .enqueue(envelope(&first_identity, Uuid::new_v4()))
        .unwrap();
    let mut other = first_identity.hook.clone();
    other.invocation_id = Uuid::new_v4();
    assert!(CodexOutbox::for_identity(path, &other).is_err());
}

#[test]
fn drain_persists_each_acknowledgement() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let identity = identity();
    for _ in 0..2 {
        outbox.enqueue(envelope(&identity, Uuid::new_v4())).unwrap();
    }
    let mut delivered = Vec::new();
    outbox
        .drain(|event| {
            delivered.push(event.sequence.0);
            Ok(())
        })
        .unwrap();
    assert_eq!(delivered, [1, 2]);
    assert!(outbox.pending().unwrap().is_empty());
}

#[test]
fn drain_does_not_hold_the_lock_during_delivery() {
    let directory = tempdir().unwrap();
    let outbox = CodexOutbox::new(directory.path().join("events.json")).unwrap();
    let identity = identity();
    outbox.enqueue(envelope(&identity, Uuid::new_v4())).unwrap();
    let mut inserted = false;
    let mut delivered = Vec::new();
    outbox
        .drain(|event| {
            delivered.push(event.sequence.0);
            if !inserted {
                inserted = true;
                outbox.enqueue(envelope(&identity, Uuid::new_v4()))?;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(delivered, [1, 2]);
    let state = outbox.load().unwrap();
    assert!(state.pending.is_empty());
    assert_eq!(state.acknowledged_sequence, EXPECTED_DRAINED_SEQUENCE);
    assert_eq!(state.next_sequence, EXPECTED_NEXT_SEQUENCE);
}

#[test]
fn persisted_acknowledged_zero_sequence_is_rejected() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("events.json");
    let identity = identity();
    let mut state = CodexOutboxState::default();
    state.owner = Some(identity.hook.clone());
    state
        .acknowledged_events
        .push_back(envelope(&identity, Uuid::new_v4()));
    crate::paths::atomic_write_json(&path, &state).unwrap();
    assert!(CodexOutbox::new(path).is_err());
}

#[test]
fn persisted_non_normalized_metadata_is_rejected() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("events.json");
    let identity = identity();
    let mut outbox = CodexOutboxState::default();
    outbox.owner = Some(identity.hook.clone());
    let mut event = envelope(&identity, Uuid::new_v4());
    event.sequence = crate::agent::hooks::EventSequence::FIRST;
    event.metadata.cwd = Some(PathBuf::from(NON_NORMAL_CWD));
    outbox.pending.push_back(event);
    outbox.next_sequence = EXPECTED_DRAINED_SEQUENCE;
    crate::paths::atomic_write_json(&path, &outbox).unwrap();
    assert!(CodexOutbox::new(path).is_err());
}

#[cfg(unix)]
#[test]
fn lock_symlink_is_rejected() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("events.json");
    let lock_path = PathBuf::from(format!("{}{}", path.display(), OUTBOX_LOCK_SUFFIX));
    let target = directory.path().join("outside.lock");
    std::fs::write(&target, b"foreign").unwrap();
    std::os::unix::fs::symlink(&target, &lock_path).unwrap();
    assert!(CodexOutbox::new(path).is_err());
}
