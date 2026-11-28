use super::super::persistence;
use super::*;
use uuid::Uuid;

const STATE_FILE: &str = "current-outbox.json";
const FIRST_QUEUE: usize = 0;
const QUEUE_FIELDS: &[&str] = &[
    stringify!(authenticated_session_start),
    stringify!(prepared_session_start),
];

fn state() -> OutboxState {
    let credentials = ClaudeHookCredentials {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
    };
    OutboxState {
        schema_version: crate::claude_hooks::CLAUDE_HOOK_OUTBOX_SCHEMA_VERSION,
        invocations: [InvocationQueue::new(credentials).unwrap()].into(),
        failed_handshakes: VecDeque::new(),
    }
}

#[test]
fn current_queue_preserves_explicit_null_lifecycle_state() {
    let original = state();
    let encoded = serde_json::to_value(&original).unwrap();
    for field in QUEUE_FIELDS {
        assert_eq!(
            encoded[stringify!(invocations)][FIRST_QUEUE][*field],
            serde_json::Value::Null
        );
    }
    let decoded: OutboxState = serde_json::from_value(encoded).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn missing_outbox_evidence_is_rejected_before_any_mutation() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(STATE_FILE);
    let original = serde_json::to_value(state()).unwrap();
    for field in std::iter::once(stringify!(failed_handshakes)).chain(QUEUE_FIELDS.iter().copied())
    {
        let mut value = original.clone();
        let target = if field == stringify!(failed_handshakes) {
            &mut value
        } else {
            &mut value[stringify!(invocations)][FIRST_QUEUE]
        };
        target.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<OutboxState>(value.clone()).is_err(),
            "{field}"
        );
        crate::paths::atomic_write_json(&path, &value).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut mutated = false;
        assert!(
            persistence::update(&path, |_| {
                mutated = true;
                Ok(())
            })
            .is_err(),
            "{field}"
        );
        assert!(!mutated);
        assert_eq!(std::fs::read(&path).unwrap(), before, "{field}");
    }
}
