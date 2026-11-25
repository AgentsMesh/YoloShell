use super::*;
use crate::{ClaudeHookState, ClaudeProcessIdentity};
use uuid::Uuid;

const AGENT_PID: u32 = 52_001;
const AGENT_START: u64 = 52_002;
const SECOND_SEQUENCE: u64 = 2;

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
fn scope_rejects_nil_runtime_and_missing_process_start() {
    let mut scope = scope();
    assert!(scope.is_valid());
    scope.agent.start_time = u64::default();
    assert!(!scope.is_valid());
    scope.agent.start_time = AGENT_START;
    scope.runtime_epoch = Uuid::nil();
    assert!(!scope.is_valid());
}

#[test]
fn state_validation_rejects_duplicate_scopes_and_gapped_sequence() {
    let scope = scope();
    let queue = || Queue {
        scope,
        acknowledged: u64::default(),
        events: VecDeque::new(),
    };
    assert!(
        Outbox {
            queues: vec![queue(), queue()]
        }
        .validate()
        .is_err()
    );
    let request = ClaudeNativeRequest {
        schema_version: crate::CLAUDE_NATIVE_SCHEMA_VERSION,
        scope,
        event_sequence: SECOND_SEQUENCE,
        event: event(),
    };
    let mut queue = queue();
    queue.events.push_back(request);
    assert!(
        Outbox {
            queues: vec![queue]
        }
        .validate()
        .is_err()
    );
}

#[test]
fn response_must_bind_scope_event_sequence_and_registered_invocation() {
    let request = ClaudeNativeRequest {
        schema_version: crate::CLAUDE_NATIVE_SCHEMA_VERSION,
        scope: scope(),
        event_sequence: crate::INITIAL_CLAUDE_EVENT_SEQUENCE,
        event: event(),
    };
    let response = crate::ClaudeNativeResponse {
        schema_version: crate::CLAUDE_NATIVE_SCHEMA_VERSION,
        accepted: true,
        code: crate::CLAUDE_HOOK_ACCEPTED.to_owned(),
        scope: request.scope,
        event_sequence: request.event_sequence,
        event_id: request.event.event_id(),
        invocation_id: Some(Uuid::new_v4()),
        revision: Some(crate::INITIAL_CLAUDE_EVENT_SEQUENCE),
        interaction: None,
    };
    assert!(response.acknowledges(&request));
    let mut foreign = response.clone();
    foreign.scope.runtime_epoch = Uuid::new_v4();
    assert!(!foreign.acknowledges(&request));
    let mut foreign = response.clone();
    foreign.event_id = Uuid::new_v4();
    assert!(!foreign.acknowledges(&request));
    let mut missing = response;
    missing.invocation_id = None;
    assert!(!missing.acknowledges(&request));
}

#[test]
fn previous_schema_requests_migrate_before_validation() {
    let scope = scope();
    let mut outbox = Outbox {
        queues: vec![Queue {
            scope,
            acknowledged: u64::default(),
            events: VecDeque::new(),
        }],
    };
    outbox.queues[0].events.push_back(ClaudeNativeRequest {
        schema_version: crate::PREVIOUS_CLAUDE_NATIVE_SCHEMA_VERSION,
        scope,
        event_sequence: crate::INITIAL_CLAUDE_EVENT_SEQUENCE,
        event: event(),
    });
    assert!(outbox.validate().is_err());
    outbox.migrate();
    assert!(outbox.validate().is_ok());
    assert_eq!(
        outbox.queues[0].events[0].schema_version,
        crate::CLAUDE_NATIVE_SCHEMA_VERSION
    );
}
