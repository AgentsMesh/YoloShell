use super::super::constants::{event_names, field_names, test_values, values};
use super::*;
use crate::agent::hooks::{EventKind, EventSequence, SessionEndReason, SessionStartSource};
use uuid::Uuid;

const UNDOCUMENTED_SOURCE: &str = crate::agent::codex::constants::FORK_ARGUMENT;

#[test]
fn wire_decode_and_mapping_preserve_structured_fields() {
    let identity = identity();
    let mut raw = wire_test_support::input_value(event_names::SESSION_START);
    raw[field_names::SOURCE] = serde_json::json!(values::RESUME);
    let value = wire_test_support::decode_value(raw).unwrap();
    let envelope = map(&value, &identity).unwrap();
    assert!(matches!(
        envelope.event,
        EventKind::SessionStart {
            source: SessionStartSource::Resume
        }
    ));
    assert!(envelope.turn_id.is_none());

    let prompt = map(
        &wire_test_support::input(event_names::USER_PROMPT_SUBMIT),
        &identity,
    )
    .unwrap();
    assert_eq!(prompt.turn_id.as_deref(), Some(TEST_TURN));
}

#[test]
fn session_start_source_contract_is_exact() {
    let identity = identity();
    for (source, expected) in [
        (values::STARTUP, SessionStartSource::Startup),
        (values::RESUME, SessionStartSource::Resume),
        (values::CLEAR, SessionStartSource::Clear),
        (values::COMPACT, SessionStartSource::Compact),
    ] {
        let mut raw = wire_test_support::input_value(event_names::SESSION_START);
        raw[field_names::SOURCE] = serde_json::json!(source);
        let value = wire_test_support::decode_value(raw).unwrap();
        assert_eq!(
            map(&value, &identity).unwrap().event,
            EventKind::SessionStart { source: expected }
        );
    }
    for source in [
        None,
        Some(UNDOCUMENTED_SOURCE),
        Some(test_values::UNKNOWN_SOURCE),
    ] {
        let mut raw = wire_test_support::input_value(event_names::SESSION_START);
        match source {
            Some(source) => raw[field_names::SOURCE] = serde_json::json!(source),
            None => {
                raw.as_object_mut().unwrap().remove(field_names::SOURCE);
            }
        }
        assert!(wire_test_support::decode_value(raw).is_err());
    }
}

#[test]
fn mapping_leaves_event_identity_for_the_locked_outbox_assignment() {
    let identity = identity();
    let value = wire_test_support::input(event_names::USER_PROMPT_SUBMIT);
    let mapped = map(&value, &identity).unwrap();
    assert_eq!(mapped.event_id, Uuid::nil());
    assert_eq!(mapped.sequence, EventSequence::ZERO);
}

#[test]
fn session_end_requires_the_documented_reason() {
    let identity = identity();
    let value = wire_test_support::input(event_names::SESSION_END);
    assert_eq!(
        map(&value, &identity).unwrap().event,
        EventKind::SessionEnd {
            reason: SessionEndReason::Other
        }
    );
    for reason in [
        None,
        Some(values::CLEAR),
        Some(values::RESUME),
        Some(test_values::UNKNOWN_REASON),
    ] {
        let mut raw = wire_test_support::input_value(event_names::SESSION_END);
        match reason {
            Some(reason) => raw[field_names::REASON] = serde_json::json!(reason),
            None => {
                raw.as_object_mut().unwrap().remove(field_names::REASON);
            }
        }
        assert!(wire_test_support::decode_value(raw).is_err());
    }
}
