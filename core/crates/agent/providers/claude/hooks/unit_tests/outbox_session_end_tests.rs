use super::*;
use crate::claude_hooks::{
    ClaudeHookCredentials, ClaudeHookOperation, ClaudeLifecycleEvent, ClaudeSessionEndReason,
};

#[path = "outbox_integrity_tests.rs"]
mod integrity_tests;

#[test]
fn session_end_reason_survives_enqueue_read_and_acknowledgement() {
    for reason in [
        ClaudeSessionEndReason::Clear,
        ClaudeSessionEndReason::Resume,
        ClaudeSessionEndReason::Other,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("outbox/state.json");
        let delivery = enqueue(&path, credentials(), event(reason))
            .unwrap()
            .unwrap();
        let request = next(&path, delivery).unwrap().unwrap();
        assert_eq!(request_reason(&request), reason);

        let persisted = persistence::read(&path, |state| Ok(state.clone())).unwrap();
        assert_eq!(
            request_reason(&persisted.invocations[0].pending[0].request),
            reason
        );
        acknowledge(&path, delivery, &request).unwrap();
        assert_eq!(next(&path, delivery).unwrap(), None);
        assert!(release_if_empty(&path, delivery).unwrap());
    }
}

#[test]
fn stale_outbox_schema_fails_closed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    enqueue(&path, credentials(), event(ClaudeSessionEndReason::Clear)).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value[crate::agent::constants::SCHEMA_VERSION_FIELD] =
        serde_json::Value::from(crate::claude_hooks::CLAUDE_HOOK_OUTBOX_SCHEMA_VERSION - 1);
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

    assert!(persistence::read(&path, |_| Ok(())).is_err());
}

fn request_reason(request: &ClaudeHookRequest) -> ClaudeSessionEndReason {
    let ClaudeHookOperation::Lifecycle {
        event: ClaudeLifecycleEvent::SessionEnd { reason, .. },
        ..
    } = request.operation
    else {
        unreachable!();
    };
    reason
}

fn credentials() -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        invocation_id: uuid::Uuid::new_v4(),
        hook_token: uuid::Uuid::new_v4(),
        helper_capability: uuid::Uuid::new_v4(),
    }
}

fn event(reason: ClaudeSessionEndReason) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::SessionEnd {
        event_id: uuid::Uuid::new_v4(),
        provider_session_id: uuid::Uuid::new_v4().hyphenated().to_string(),
        reason,
    }
}
