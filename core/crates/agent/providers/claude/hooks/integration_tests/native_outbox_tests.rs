use super::*;
use crate::{ClaudeHookState, ClaudeProcessIdentity};
use uuid::Uuid;

#[path = "../test_support/native_outbox.rs"]
mod fixture;

const AGENT_PID: u32 = 53_001;
const AGENT_START: u64 = 53_002;
const OUTBOX_FILE: &str = "native-outbox.json";
const HELPER_COUNT: usize = 2;

fn scope() -> ClaudeNativeScope {
    ClaudeNativeScope {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        agent: ClaudeProcessIdentity {
            pid: AGENT_PID,
            start_time: AGENT_START,
        },
    }
}

fn event() -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::StateChanged {
        event_id: Uuid::new_v4(),
        provider_session_id: Uuid::new_v4().to_string(),
        state: ClaudeHookState::Running,
    }
}

#[test]
fn lost_ack_preserves_exact_event_and_duplicate_ack_is_idempotent() {
    let directory = fixture::private_directory();
    let path = directory.path().join(OUTBOX_FILE);
    let scope = scope();
    enqueue(&path, scope, event()).unwrap();
    let first = next(&path, scope).unwrap().unwrap();
    assert_eq!(next(&path, scope).unwrap().unwrap(), first);
    acknowledge(&path, &first).unwrap();
    acknowledge(&path, &first).unwrap();
    assert!(next(&path, scope).unwrap().is_none());
}

#[test]
fn concurrent_helpers_reserve_consecutive_durable_sequence_numbers() {
    let directory = fixture::private_directory();
    let path = directory.path().join(OUTBOX_FILE);
    let scope = scope();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(HELPER_COUNT));
    let handles = (0..HELPER_COUNT)
        .map(|_| {
            let barrier = barrier.clone();
            let path = path.clone();
            std::thread::spawn(move || {
                barrier.wait();
                enqueue(&path, scope, event()).unwrap();
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }
    for expected in crate::INITIAL_CLAUDE_EVENT_SEQUENCE..=HELPER_COUNT as u64 {
        let request = next(&path, scope).unwrap().unwrap();
        assert_eq!(request.event_sequence, expected);
        acknowledge(&path, &request).unwrap();
    }
    assert!(next(&path, scope).unwrap().is_none());
}

#[test]
fn conflicting_event_identity_cannot_replace_a_persisted_occurrence() {
    let directory = fixture::private_directory();
    let path = directory.path().join(OUTBOX_FILE);
    let scope = scope();
    let initial = event();
    enqueue(&path, scope, initial.clone()).unwrap();
    let mut replacement = initial.clone();
    if let ClaudeLifecycleEvent::StateChanged { state, .. } = &mut replacement {
        *state = ClaudeHookState::WaitingForInput;
    }
    assert!(enqueue(&path, scope, replacement).is_err());
    assert_eq!(next(&path, scope).unwrap().unwrap().event, initial);
}

