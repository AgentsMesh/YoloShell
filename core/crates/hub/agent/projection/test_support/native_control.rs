use super::constants::*;
use yolo_agent_control_contract::*;
use yolo_hub_agent_protocol::*;
use yolo_terminal_model::TerminalView;

pub fn observation(view: &TerminalView, revision: u64) -> NativeControlObservation {
    let binding = view.agent.as_ref().unwrap().clone();
    let runtime = view.runtime.as_ref().unwrap();
    let witness = NativeControlWitness::new(NativeControlWitnessFields {
        terminal_id: view.manifest.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        boot_id: CONTROL_BOOT.into(),
        kind: binding.kind.clone(),
        process_pid: binding.process_pid.unwrap(),
        process_start_time: binding.process_start_time.unwrap(),
        binding_digest: binding_digest(&binding).unwrap(),
        authority_digest: authority_digest(&(revision, runtime.worker_id)).unwrap(),
    })
    .unwrap();
    NativeControlObservation::new(binding, witness, false).unwrap()
}
pub fn observation_with_interaction(
    view: &TerminalView,
    revision: u64,
    interaction: uuid::Uuid,
) -> NativeControlObservation {
    let base = observation(view, revision);
    let pending = NativeInteraction::new(
        interaction,
        NativeInteractionKind::Approval,
        vec![
            NativeInteractionChoice {
                id: "allow".into(),
                label: "Allow".into(),
            },
            NativeInteractionChoice {
                id: "deny".into(),
                label: "Deny".into(),
            },
        ],
        "Bash: cargo test".into(),
    )
    .unwrap();
    NativeControlObservation::new_with_interaction(
        base.binding().clone(),
        base.witness().clone(),
        false,
        None,
        Some(pending),
    )
    .unwrap()
}

pub fn respond_request(snapshot: &AgentSnapshot) -> AgentControlRequest {
    let interaction = snapshot.fields().interaction.as_ref().unwrap();
    AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        target: snapshot.cursor().target(),
        expected_cursor: snapshot.cursor(),
        action: AgentAction::Respond {
            interaction: interaction.reference(),
            response: InteractionResponse::Choice {
                choice: "allow".into(),
            },
        },
    })
    .unwrap()
}

pub fn request(snapshot: &AgentSnapshot) -> AgentControlRequest {
    AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        target: snapshot.cursor().target(),
        expected_cursor: snapshot.cursor(),
        action: AgentAction::Interrupt {},
    })
    .unwrap()
}
