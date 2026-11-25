use super::constants::*;
use uuid::Uuid;
use yolo_agent_contract::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_agent_control_contract::*;

pub fn binding() -> AgentBinding {
    AgentBinding {
        schema_version: yolo_agent_adapter_contract::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: AgentKind::Codex,
        provider_session_id: NATIVE_SESSION.into(),
        executable: EXECUTABLE.into(),
        cwd: CWD.into(),
        evidence_path: EVIDENCE.into(),
        data_home: DATA_HOME.into(),
        process_pid: Some(PID),
        process_start_time: Some(START_TIME),
        state: AgentState::Running,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: DETECTED_MS,
        updated_at_ms: UPDATED_MS,
    }
}
pub fn witness_fields(binding: &AgentBinding) -> NativeControlWitnessFields {
    NativeControlWitnessFields {
        terminal_id: binding.terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        boot_id: BOOT_ID.into(),
        kind: binding.kind.clone(),
        process_pid: binding.process_pid.unwrap(),
        process_start_time: binding.process_start_time.unwrap(),
        binding_digest: binding_digest(binding).unwrap(),
        authority_digest: authority_digest(&REVISION).unwrap(),
    }
}
pub fn observation() -> NativeControlObservation {
    let binding = binding();
    let witness = NativeControlWitness::new(witness_fields(&binding)).unwrap();
    NativeControlObservation::new(binding, witness, false).unwrap()
}
