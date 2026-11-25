use super::test_support::{Fixture, prompt, request};
use crate::codex_hooks::{CodexNativeAck, CodexNativeAckCode, CodexNativeRequest};

#[test]
fn raw_callbacks_round_trip_without_inventing_nullable_agent_fields() {
    let fixture = Fixture::new();
    for callback in [request(&fixture), prompt(&fixture)] {
        let encoded = serde_json::to_vec(&callback).unwrap();
        let decoded: CodexNativeRequest = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, callback);
        decoded.validate().unwrap();
        assert_eq!(
            decoded.envelope(&fixture.identity).unwrap().event_id,
            callback.event_id
        );
    }
}

#[test]
fn native_request_rejects_scope_and_path_substitution() {
    let fixture = Fixture::new();
    let original = request(&fixture);
    for changed in [
        {
            let mut value = original.clone();
            value.terminal_id = uuid::Uuid::nil();
            value
        },
        {
            let mut value = original.clone();
            value.runtime_epoch = uuid::Uuid::nil();
            value
        },
        {
            let mut value = original.clone();
            value.event_id = uuid::Uuid::nil();
            value
        },
        {
            let mut value = original.clone();
            value.data_home_candidates.clear();
            value
        },
        {
            let mut value = original.clone();
            value.hook_executable = Default::default();
            value
        },
    ] {
        assert!(changed.validate().is_err());
    }
    let mut changed = original;
    changed.runtime_epoch = uuid::Uuid::new_v4();
    assert!(changed.envelope(&fixture.identity).is_err());
}

#[test]
fn native_ack_rejects_cross_occurrence_and_success_code_forgery() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    CodexNativeAck::accepted(&callback, false)
        .validate_for(&callback)
        .unwrap();
    CodexNativeAck::accepted(&callback, true)
        .validate_for(&callback)
        .unwrap();
    let mut ack = CodexNativeAck::accepted(&callback, false);
    ack.event_id = uuid::Uuid::new_v4();
    assert!(ack.validate_for(&callback).is_err());
    assert!(
        CodexNativeAck::rejected(&callback, CodexNativeAckCode::Accepted)
            .validate_for(&callback)
            .is_err()
    );
    CodexNativeAck::rejected(&callback, CodexNativeAckCode::NotReady)
        .validate_for(&callback)
        .unwrap();
}
