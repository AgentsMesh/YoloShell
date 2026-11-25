use super::*;
use crate::claude_hooks::{ClaudeHookState, ClaudeLifecycleEvent};
use crate::process::{ProcessIdentity, ProcessIdentityRelation};
use std::sync::{Arc, Barrier};
use uuid::Uuid;

const FAKE_AGENT_PID: u32 = 71_001;
const FAKE_AGENT_START: u64 = 71_002;
const FAKE_HELPER_PID: u32 = 72_001;
const FAKE_HELPER_START: u64 = 72_002;

#[test]
fn concurrent_helpers_create_one_delivery_owner_and_preserve_both_events() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    let credentials = credentials();
    let barrier = Arc::new(Barrier::new(2));
    let handles = [Uuid::new_v4(), Uuid::new_v4()].map(|event_id| {
        let barrier = Arc::clone(&barrier);
        let path = path.clone();
        std::thread::spawn(move || {
            barrier.wait();
            enqueue_state(&path, credentials, event(event_id)).unwrap()
        })
    });
    let deliveries = handles
        .into_iter()
        .filter_map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(deliveries.len(), 1);
    let delivery = deliveries[0];
    let mut delivered = 0;
    while let Some(request) = next(&path, delivery).unwrap() {
        acknowledge(&path, delivery, &request).unwrap();
        delivered += 1;
    }
    assert_eq!(delivered, 2);
    assert!(release_if_empty(&path, delivery).unwrap());
}

#[test]
fn owner_observes_an_event_enqueued_while_it_is_delivering() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    let credentials = credentials();
    let delivery = enqueue_state(&path, credentials, event(Uuid::new_v4()))
        .unwrap()
        .unwrap();
    let first = next(&path, delivery).unwrap().unwrap();
    assert!(
        enqueue_state(&path, credentials, event(Uuid::new_v4()))
            .unwrap()
            .is_none()
    );
    acknowledge(&path, delivery, &first).unwrap();
    let second = next(&path, delivery).unwrap().unwrap();
    acknowledge(&path, delivery, &second).unwrap();
    assert!(release_if_empty(&path, delivery).unwrap());
}

#[test]
fn a_gone_owner_is_replaced_but_an_unknown_owner_is_not() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    let credentials = credentials();
    enqueue_state(&path, credentials, event(Uuid::new_v4()))
        .unwrap()
        .unwrap();
    set_claim(&path, fake_helper());
    let stale = Delivery {
        invocation_id: credentials.invocation_id,
        owner: ProcessClaim::from_identity(fake_helper()),
    };
    let gone = crate::process::install_test_process_identity_relation(
        FAKE_HELPER_PID,
        FAKE_HELPER_START,
        ProcessIdentityRelation::Gone,
    );
    let recovered = enqueue_state(&path, credentials, event(Uuid::new_v4()))
        .unwrap()
        .unwrap();
    assert!(next(&path, stale).is_err());
    assert!(next(&path, recovered).unwrap().is_some());
    drop(gone);

    set_claim(&path, fake_helper());
    let _unknown = crate::process::install_test_process_identity_relation(
        FAKE_HELPER_PID,
        FAKE_HELPER_START,
        ProcessIdentityRelation::Unknown,
    );
    assert!(
        enqueue_state(&path, credentials, event(Uuid::new_v4()))
            .unwrap()
            .is_none()
    );
    let state = snapshot(&path);
    assert_eq!(
        state.invocations[0].claim.unwrap().identity(),
        fake_helper()
    );
}

#[test]
fn unknown_empty_agent_queue_is_retained_until_identity_is_authoritatively_gone() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    let first = credentials();
    enqueue_for_recovery(&path, first, event(Uuid::new_v4()), None, fake_agent()).unwrap();
    persistence::update(&path, |state| {
        let queue = &mut state.invocations[0];
        queue.acknowledged_sequence = queue.next_sequence - 1;
        queue.pending.clear();
        Ok(())
    })
    .unwrap();
    let unknown = crate::process::install_test_process_identity_relation(
        FAKE_AGENT_PID,
        FAKE_AGENT_START,
        ProcessIdentityRelation::Unknown,
    );
    enqueue_state(&path, credentials(), event(Uuid::new_v4())).unwrap();
    assert_eq!(snapshot(&path).invocations.len(), 2);
    drop(unknown);
    let _gone = crate::process::install_test_process_identity_relation(
        FAKE_AGENT_PID,
        FAKE_AGENT_START,
        ProcessIdentityRelation::Gone,
    );
    enqueue_state(&path, credentials(), event(Uuid::new_v4())).unwrap();
    let state = snapshot(&path);
    assert_eq!(state.invocations.len(), 2);
    assert!(
        state
            .invocations
            .iter()
            .all(|queue| queue.credentials != first)
    );
}

fn enqueue_state(
    path: &std::path::Path,
    credentials: ClaudeHookCredentials,
    event: ClaudeLifecycleEvent,
) -> Result<Option<Delivery>> {
    persistence::update(path, |state| {
        state::enqueue(state, credentials, event, None)
    })
}

fn set_claim(path: &std::path::Path, identity: ProcessIdentity) {
    persistence::update(path, |state| {
        state.invocations[0].claim = Some(ProcessClaim::from_identity(identity));
        Ok(())
    })
    .unwrap();
}

fn snapshot(path: &std::path::Path) -> types::OutboxState {
    persistence::read(path, |state| Ok(state.clone())).unwrap()
}

fn credentials() -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
    }
}

fn event(event_id: Uuid) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::StateChanged {
        event_id,
        provider_session_id: Uuid::new_v4().hyphenated().to_string(),
        state: ClaudeHookState::Running,
    }
}

fn fake_agent() -> ProcessIdentity {
    ProcessIdentity {
        pid: FAKE_AGENT_PID,
        start_time: FAKE_AGENT_START,
    }
}

fn fake_helper() -> ProcessIdentity {
    ProcessIdentity {
        pid: FAKE_HELPER_PID,
        start_time: FAKE_HELPER_START,
    }
}
