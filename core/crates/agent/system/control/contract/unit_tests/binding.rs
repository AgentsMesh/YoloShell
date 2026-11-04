use crate::{constants::*, fixture::*};
use yolo_agent_contract::{AgentBinding, AgentKind, AgentState};
use yolo_agent_control_contract::*;

#[test]
fn binding_validation_reuses_schema_lifecycle_timestamps_and_required_process_rules() {
    let changes: &[fn(&mut AgentBinding)] = &[
        |b| b.schema_version += STEP,
        |b| b.provider_session_id = PADDED_ID.into(),
        |b| b.detected_at_ms = -DETECTED_MS,
        |b| b.updated_at_ms = b.detected_at_ms - i64::from(STEP),
        |b| b.process_pid = None,
        |b| b.process_start_time = None,
        |b| b.state = AgentState::Exited,
    ];
    for change in changes {
        let mut binding = binding();
        change(&mut binding);
        assert_eq!(
            binding_digest(&binding),
            Err(ControlContractError::InvalidBinding)
        );
    }
    let mut invalid = binding();
    invalid.terminal_id = uuid::Uuid::nil();
    assert_eq!(
        binding_digest(&invalid),
        Err(ControlContractError::InvalidIdentity)
    );
    invalid = binding();
    invalid.kind = AgentKind::Other("Third Party".into());
    assert_eq!(
        binding_digest(&invalid),
        Err(ControlContractError::UnsupportedProvider)
    );
    let mut unknown = binding();
    unknown.kind = AgentKind::Other(OTHER_PROVIDER.into());
    assert!(binding_digest(&unknown).is_ok());
    invalid = binding();
    invalid.process_pid = Some(MAX_PROCESS_PID + STEP);
    assert_eq!(
        binding_digest(&invalid),
        Err(ControlContractError::InvalidProcess)
    );
}

#[test]
fn binding_metadata_is_bounded_without_using_reported_paths_as_filesystem_authority() {
    let changes: &[fn(&mut AgentBinding)] = &[
        |b| b.cwd = RELATIVE_PATH.into(),
        |b| b.data_home = RELATIVE_PATH.into(),
        |b| b.evidence_path = RELATIVE_PATH.into(),
        |b| b.executable = String::new(),
        |b| b.provider_session_id = CONTROL_TEXT.into(),
        |b| b.cwd = CONTROL_TEXT.into(),
        |b| b.data_home = CONTROL_TEXT.into(),
        |b| b.evidence_path = CONTROL_TEXT.into(),
        |b| {
            b.executable =
                TEXT_CHARACTER.repeat(MAX_BINDING_TEXT_BYTES + usize::try_from(STEP).unwrap())
        },
    ];
    for change in changes {
        let mut binding = binding();
        change(&mut binding);
        assert_eq!(
            binding_digest(&binding),
            Err(ControlContractError::InvalidBinding)
        );
    }
    let mut boundary = binding();
    boundary.cwd =
        PATH_PREFIX.to_owned() + &TEXT_CHARACTER.repeat(MAX_BINDING_TEXT_BYTES - PATH_PREFIX.len());
    binding_digest(&boundary).unwrap();
    boundary.cwd.push_str(TEXT_CHARACTER);
    assert_eq!(
        binding_digest(&boundary),
        Err(ControlContractError::InvalidBinding)
    );
    let observation = observation();
    assert_eq!(observation.binding().cwd, CWD);
}

#[test]
fn exited_or_processless_unknown_bindings_cannot_match_a_positive_process_witness() {
    for state in [AgentState::Exited, AgentState::Unknown] {
        let mut binding = binding();
        let witness = NativeControlWitness::new(witness_fields(&binding)).unwrap();
        binding.state = state;
        binding.process_pid = None;
        binding.process_start_time = None;
        assert!(binding_digest(&binding).is_ok());
        assert_eq!(
            NativeControlObservation::new(binding, witness, false),
            Err(ControlContractError::BindingMismatch)
        );
    }
}
