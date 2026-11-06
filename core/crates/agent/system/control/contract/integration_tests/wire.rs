use crate::{constants::*, fixture::*};
use uuid::Uuid;
use yolo_agent_contract::AgentKind;
use yolo_agent_control_contract::*;

#[test]
fn strict_native_observation_round_trips_for_each_builtin_and_rejects_future_schema() {
    for kind in [AgentKind::Codex, AgentKind::ClaudeCode] {
        let mut binding = binding();
        binding.kind = kind;
        let witness = NativeControlWitness::new(witness_fields(&binding)).unwrap();
        let expected = NativeControlObservation::new(binding, witness, false).unwrap();
        let encoded = serde_json::to_value(&expected).unwrap();
        assert_eq!(
            serde_json::from_value::<NativeControlObservation>(encoded.clone()).unwrap(),
            expected
        );
        let mut future = encoded;
        future[SCHEMA_FIELD] = serde_json::json!(NATIVE_CONTROL_SCHEMA_VERSION + SCHEMA_STEP);
        assert!(serde_json::from_value::<NativeControlObservation>(future).is_err());
    }
}

#[test]
fn native_observation_carries_a_strict_pending_interaction() {
    let binding = binding();
    let witness = NativeControlWitness::new(witness_fields(&binding)).unwrap();
    let interaction = NativeInteraction::new(
        Uuid::new_v4(),
        NativeInteractionKind::Approval,
        vec![NativeInteractionChoice {
            id: "allow".into(),
            label: "Allow".into(),
        }],
        "Bash: cargo test".into(),
    )
    .unwrap();
    let expected = NativeControlObservation::new_with_interaction(
        binding,
        witness,
        false,
        None,
        Some(interaction),
    )
    .unwrap();
    let encoded = serde_json::to_value(&expected).unwrap();
    assert_eq!(
        serde_json::from_value::<NativeControlObservation>(encoded.clone()).unwrap(),
        expected
    );
    let mut extra = encoded.clone();
    extra["interaction"]["extra"] = serde_json::json!(PRIVATE_ERROR);
    assert!(serde_json::from_value::<NativeControlObservation>(extra).is_err());
    let mut nil = encoded;
    nil["interaction"]["id"] = serde_json::json!(Uuid::nil().to_string());
    assert!(serde_json::from_value::<NativeControlObservation>(nil).is_err());
}

#[test]
fn native_envelope_never_accepts_extra_commands_hook_credentials_or_writable_snapshots() {
    let encoded = serde_json::to_value(observation()).unwrap();
    for key in [COMMAND_FIELD, CREDENTIAL_FIELD, SNAPSHOT_FIELD] {
        let mut outer = encoded.clone();
        outer[key] = serde_json::json!(PRIVATE_ERROR);
        assert!(serde_json::from_value::<NativeControlObservation>(outer).is_err());
        let mut witness = encoded.clone();
        witness[WITNESS_FIELD][key] = serde_json::json!(PRIVATE_ERROR);
        assert!(serde_json::from_value::<NativeControlObservation>(witness).is_err());
        let mut binding = encoded.clone();
        binding[BINDING_FIELD][key] = serde_json::json!(PRIVATE_ERROR);
        assert!(serde_json::from_value::<NativeControlObservation>(binding).is_err());
    }
}

#[test]
fn uuid_and_digest_wire_contracts_reject_noncanonical_nil_missing_or_wrong_typed_values() {
    let encoded = serde_json::to_value(observation()).unwrap();
    for key in [TERMINAL_FIELD, RUNTIME_FIELD, WORKER_FIELD] {
        for value in [Uuid::nil().to_string(), CANONICAL_ID.to_uppercase()] {
            let mut changed = encoded.clone();
            changed[WITNESS_FIELD][key] = serde_json::json!(value);
            assert!(serde_json::from_value::<NativeControlObservation>(changed).is_err());
        }
        let mut missing = encoded.clone();
        missing[WITNESS_FIELD].as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<NativeControlObservation>(missing).is_err());
    }
    for key in [BINDING_DIGEST_FIELD, AUTHORITY_DIGEST_FIELD] {
        let mut invalid = encoded.clone();
        invalid[WITNESS_FIELD][key] = serde_json::json!(PRIVATE_ERROR);
        assert!(serde_json::from_value::<NativeControlObservation>(invalid).is_err());
    }
    let mut nil_binding = encoded.clone();
    nil_binding[BINDING_FIELD][TERMINAL_FIELD] = serde_json::json!(Uuid::nil());
    assert!(serde_json::from_value::<NativeControlObservation>(nil_binding).is_err());
    let mut wrong_pid = encoded;
    wrong_pid[WITNESS_FIELD][PID_FIELD] = serde_json::json!(-i64::from(STEP));
    assert!(serde_json::from_value::<NativeControlObservation>(wrong_pid).is_err());
}
