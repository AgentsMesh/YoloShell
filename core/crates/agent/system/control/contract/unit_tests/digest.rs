use crate::{constants::*, fixture::*};
use serde::{Serialize, Serializer};
use std::error::Error;
use yolo_agent_contract::{AgentBinding, AgentRecoveryAvailability, AgentState};
use yolo_agent_control_contract::*;
use yolo_crypto::Sha256Digest;

#[test]
fn identical_binding_round_trip_has_stable_hash_and_authority_domain_is_distinct() {
    let binding = binding();
    let image = serde_json::to_value(&binding).unwrap();
    let restored: AgentBinding = serde_json::from_value(image.clone()).unwrap();
    let expected = binding_digest(&binding).unwrap();
    assert_eq!(binding_digest(&restored).unwrap(), expected);
    assert_ne!(authority_digest(&image).unwrap(), expected);
    assert_ne!(Sha256Digest::capture(&image).unwrap(), expected);
    assert_ne!(
        authority_digest(&REVISION).unwrap(),
        authority_digest(&(REVISION + TIME_STEP)).unwrap()
    );
}

#[test]
fn every_execution_and_metadata_change_invalidates_previous_binding_digest() {
    let original = binding();
    let expected = binding_digest(&original).unwrap();
    let changes: &[fn(&mut AgentBinding)] = &[
        |b| b.terminal_id = uuid::Uuid::new_v4(),
        |b| b.provider_session_id.push_str(TEXT_CHARACTER),
        |b| b.process_pid = b.process_pid.map(|pid| pid + STEP),
        |b| b.process_start_time = b.process_start_time.map(|start| start + TIME_STEP),
        |b| b.cwd = OTHER_CWD.into(),
        |b| b.data_home = OTHER_CWD.into(),
        |b| b.evidence_path = OTHER_EVIDENCE.into(),
        |b| b.executable = OTHER_EXECUTABLE.into(),
        |b| b.state = AgentState::WaitingForInput,
        |b| b.recovery_availability = AgentRecoveryAvailability::Unavailable,
        |b| b.detected_at_ms += i64::from(STEP),
        |b| b.updated_at_ms += i64::from(STEP),
    ];
    for change in changes {
        let mut changed = original.clone();
        change(&mut changed);
        assert_ne!(binding_digest(&changed).unwrap(), expected);
    }
}

#[test]
fn failed_authority_serialization_never_exposes_private_guard_evidence() {
    struct RejectedEvidence;
    impl Serialize for RejectedEvidence {
        fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(PRIVATE_ERROR))
        }
    }
    let error = authority_digest(&RejectedEvidence).unwrap_err();
    assert_eq!(error, ControlContractError::Digest);
    assert!(!error.to_string().contains(PRIVATE_ERROR));
    assert!(error.source().is_none());
}
