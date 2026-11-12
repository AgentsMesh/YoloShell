use super::*;
use crate::claude_hooks::{
    ClaudeHookOperation, ClaudeHookState, ClaudeLifecycleEvent, MAX_CLAUDE_OUTBOX_EVENTS,
    MAX_CLAUDE_OUTBOX_INVOCATIONS,
};
use crate::process::ProcessIdentity;
use uuid::Uuid;

const FAKE_AGENT_PID: u32 = 73_001;
const FAKE_AGENT_START: u64 = 73_002;

#[test]
fn invocation_and_event_capacities_fail_without_discarding_durable_events() {
    let directory = tempfile::tempdir().unwrap();
    let invocation_path = directory.path().join("invocations/state.json");
    for _ in 0..MAX_CLAUDE_OUTBOX_INVOCATIONS {
        enqueue_for_recovery(&invocation_path, credentials(), event(), None, fake_agent()).unwrap();
    }
    assert!(
        enqueue_for_recovery(&invocation_path, credentials(), event(), None, fake_agent(),)
            .is_err()
    );
    assert_eq!(
        snapshot(&invocation_path).invocations.len(),
        MAX_CLAUDE_OUTBOX_INVOCATIONS
    );

    let event_path = directory.path().join("events/state.json");
    let credentials = credentials();
    for _ in 0..MAX_CLAUDE_OUTBOX_EVENTS {
        enqueue_for_recovery(&event_path, credentials, event(), None, fake_agent()).unwrap();
    }
    assert!(enqueue_for_recovery(&event_path, credentials, event(), None, fake_agent()).is_err());
    assert_eq!(
        snapshot(&event_path).invocations[0].pending.len(),
        MAX_CLAUDE_OUTBOX_EVENTS
    );
}

#[test]
fn gone_unauthenticated_handshakes_fold_into_bounded_audit_and_release_capacity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("failed/state.json");
    for _ in 0..MAX_CLAUDE_OUTBOX_INVOCATIONS {
        enqueue_for_recovery(&path, credentials(), event(), None, fake_agent()).unwrap();
    }
    let _gone = crate::process::install_test_process_identity_relation(
        FAKE_AGENT_PID,
        FAKE_AGENT_START,
        crate::process::ProcessIdentityRelation::Gone,
    );
    persistence::update(&path, |state| {
        retirement::prune_dead(state);
        Ok(())
    })
    .unwrap();
    let retired = snapshot(&path);
    assert!(retired.invocations.is_empty());
    assert_eq!(
        retired.failed_handshakes.len(),
        crate::claude_hooks::MAX_CLAUDE_FAILED_HANDSHAKE_AUDITS
    );

    enqueue_for_recovery(&path, credentials(), event(), None, fake_agent()).unwrap();
    assert_eq!(snapshot(&path).invocations.len(), 1);
}

#[test]
fn event_sequence_overflow_fails_before_mutating_the_queue() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    let credentials = credentials();
    enqueue_for_recovery(&path, credentials, event(), None, fake_agent()).unwrap();
    persistence::update(&path, |state| {
        let queue = &mut state.invocations[0];
        queue.pending.clear();
        queue.acknowledged_sequence = u64::MAX - 1;
        queue.next_sequence = u64::MAX;
        Ok(())
    })
    .unwrap();
    let _retained = crate::process::install_test_process_identity_relation(
        FAKE_AGENT_PID,
        FAKE_AGENT_START,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert!(
        persistence::update(&path, |state| {
            state::enqueue(state, credentials, event(), None)
        })
        .is_err()
    );
    let queue = &snapshot(&path).invocations[0];
    assert!(queue.pending.is_empty());
    assert_eq!(queue.next_sequence, u64::MAX);
}

#[test]
fn acknowledgement_requires_the_exact_front_payload() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    let credentials = credentials();
    let delivery = persistence::update(&path, |state| {
        state::enqueue(state, credentials, event(), None)
    })
    .unwrap()
    .unwrap();
    let accepted = next(&path, delivery).unwrap().unwrap();
    let mut altered = accepted.clone();
    let ClaudeHookOperation::Lifecycle { event, .. } = &mut altered.operation else {
        unreachable!();
    };
    let ClaudeLifecycleEvent::StateChanged { event_id, .. } = event else {
        unreachable!();
    };
    *event_id = Uuid::new_v4();
    assert!(acknowledge(&path, delivery, &altered).is_err());
    assert_eq!(next(&path, delivery).unwrap(), Some(accepted));
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

fn event() -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::StateChanged {
        event_id: Uuid::new_v4(),
        provider_session_id: Uuid::new_v4().hyphenated().to_string(),
        state: ClaudeHookState::WaitingForInput,
    }
}

fn fake_agent() -> ProcessIdentity {
    ProcessIdentity {
        pid: FAKE_AGENT_PID,
        start_time: FAKE_AGENT_START,
    }
}
