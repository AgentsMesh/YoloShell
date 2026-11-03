use super::{envelope, identity};
use crate::codex_hooks::CodexOutbox;
use crate::codex_hooks::constants::{FIRST_SEQUENCE, OUTBOX_FILE_NAME};
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier, Mutex};
use uuid::Uuid;

const INVOCATION_DIRECTORY: &str = "competing-invocation";
const RETAINED_DIRECTORY: &str = "competing-retained-invocation";
const SECOND_SEQUENCE: u64 = FIRST_SEQUENCE + FIRST_SEQUENCE;
const HELPER_COUNT: usize = 2;

#[test]
fn replacement_helper_commits_while_retained_transaction_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let invocation = root.path().join(INVOCATION_DIRECTORY);
    let retained = root.path().join(RETAINED_DIRECTORY);
    let outbox_path = invocation.join(OUTBOX_FILE_NAME);
    let retained_path = retained.join(OUTBOX_FILE_NAME);
    let original = CodexOutbox::new(&outbox_path).unwrap();
    let original_owner = identity();
    let original_id = Uuid::new_v4();
    original
        .enqueue(envelope(&original_owner, original_id))
        .unwrap();

    let retry_id = Uuid::new_v4();
    let retry = envelope(&original_owner, retry_id);
    let helper_id = Uuid::new_v4();
    let helper = envelope(&identity(), helper_id);
    let helper_commit = Arc::new(Mutex::new(None));
    let captured_commit = Arc::clone(&helper_commit);
    let invocation_for_hook = invocation.clone();
    let retained_for_hook = retained.clone();
    let outbox_for_hook = outbox_path.clone();
    super::outbox_storage::set_after_load_hook(move || {
        std::fs::rename(invocation_for_hook, retained_for_hook).unwrap();
        let replacement = CodexOutbox::new(&outbox_for_hook).unwrap();
        replacement.enqueue(helper).unwrap();
        *captured_commit.lock().unwrap() = Some(std::fs::read(outbox_for_hook).unwrap());
    });

    assert!(original.enqueue(retry.clone()).is_err());
    let committed = helper_commit.lock().unwrap().clone().unwrap();
    assert_eq!(std::fs::read(&outbox_path).unwrap(), committed);

    let retained_outbox = CodexOutbox::new(&retained_path).unwrap();
    let retried = retained_outbox.enqueue(retry).unwrap();
    assert_eq!(retried.sequence.0, SECOND_SEQUENCE);
    let retained_state = retained_outbox.load().unwrap();
    assert_sequences_and_ids(
        &retained_state.pending,
        &[FIRST_SEQUENCE, SECOND_SEQUENCE],
        &[original_id, retry_id],
    );

    let replacement_state = CodexOutbox::new(&outbox_path).unwrap().load().unwrap();
    assert_sequences_and_ids(&replacement_state.pending, &[FIRST_SEQUENCE], &[helper_id]);
}

#[test]
fn same_directory_helpers_allocate_unique_ids_and_contiguous_sequences() {
    let root = tempfile::tempdir().unwrap();
    let outbox_path = root
        .path()
        .join(INVOCATION_DIRECTORY)
        .join(OUTBOX_FILE_NAME);
    let owner = identity();
    let first = CodexOutbox::for_identity(&outbox_path, &owner.hook).unwrap();
    let second = CodexOutbox::for_identity(&outbox_path, &owner.hook).unwrap();
    let first_event = envelope(&owner, Uuid::nil());
    let second_event = envelope(&owner, Uuid::nil());
    let barrier = Arc::new(Barrier::new(HELPER_COUNT));

    let (first_result, second_result) = std::thread::scope(|scope| {
        let first_barrier = Arc::clone(&barrier);
        let first_thread = scope.spawn(move || {
            first_barrier.wait();
            first.enqueue(first_event).unwrap()
        });
        let second_thread = scope.spawn(move || {
            barrier.wait();
            second.enqueue(second_event).unwrap()
        });
        (first_thread.join().unwrap(), second_thread.join().unwrap())
    });

    let mut allocated = [first_result.sequence.0, second_result.sequence.0];
    allocated.sort_unstable();
    assert_eq!(allocated, [FIRST_SEQUENCE, SECOND_SEQUENCE]);
    assert!(!first_result.event_id.is_nil());
    assert!(!second_result.event_id.is_nil());
    assert_ne!(first_result.event_id, second_result.event_id);
    let state = CodexOutbox::for_identity(&outbox_path, &owner.hook)
        .unwrap()
        .load()
        .unwrap();
    assert_sequences_and_ids(
        &state.pending,
        &[FIRST_SEQUENCE, SECOND_SEQUENCE],
        &[first_result.event_id, second_result.event_id],
    );
}

fn assert_sequences_and_ids(
    events: &std::collections::VecDeque<crate::agent::hooks::HookEnvelope>,
    expected_sequences: &[u64],
    expected_ids: &[Uuid],
) {
    let sequences: Vec<_> = events.iter().map(|event| event.sequence.0).collect();
    let ids: BTreeSet<_> = events.iter().map(|event| event.event_id).collect();
    assert_eq!(sequences, expected_sequences);
    assert_eq!(ids.len(), events.len());
    assert_eq!(ids, expected_ids.iter().copied().collect());
}
