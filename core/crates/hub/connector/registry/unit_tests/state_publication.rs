use super::{constants::*, support::*};
use crate::state::State;
use yolo_hub_connector_protocol::EnrollmentStatus;
use yolo_hub_connector_store::StoredState;
use yolo_hub_identity::{ConnectorGeneration, ConnectorId, ExecutionId, RuntimeEpoch};

#[test]
fn retained_publication_requires_exact_connector_generation_execution_terminal_and_payload() {
    let mutations: &[fn(&mut yolo_hub_connector_protocol::Publication)] = &[
        |p| p.connector_id = ConnectorId::new(),
        |p| p.generation = ConnectorGeneration::new(),
        |p| p.agent.execution_id = ExecutionId::new(),
        |p| p.terminal.runtime_epoch = RuntimeEpoch::new(),
        |p| p.sequence += STEP,
        |p| {
            p.state.working_directory = Some(
                yolo_hub_agent_protocol::AgentWorkingDirectory::new(OTHER_DIRECTORY.into())
                    .unwrap(),
            )
        },
    ];
    for mutate in mutations {
        let (seed, mut state) = published();
        mutate(state.last_publication.as_mut().unwrap());
        assert!(state.validate(&seed).is_err());
    }
    let (seed, mut state) = published();
    state.last_sequence += STEP;
    assert!(state.validate(&seed).is_err());
}

#[test]
fn revoked_or_superseded_snapshots_must_be_stale_and_cannot_recover_native_authority() {
    let (seed, mut state) = published();
    state.status = EnrollmentStatus::Superseded;
    assert!(state.validate(&seed).is_err());
    crate::observations::stale(&mut state).unwrap();
    state.validate(&seed).unwrap();
    let mut initial = State::initial();
    initial.context = state.context;
    initial.validate(&seed).unwrap();
    initial.last_sequence = STEP;
    assert!(initial.validate(&seed).is_err());
}
