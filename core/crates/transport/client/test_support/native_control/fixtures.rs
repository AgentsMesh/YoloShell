use super::*;

pub(super) fn target() -> RuntimeRef {
    RuntimeRef::new(Uuid::new_v4(), Uuid::new_v4())
}

pub(super) fn observation(target: RuntimeRef, state: AgentState) -> NativeControlObservation {
    let binding = AgentBinding {
        schema_version: yolo_runtime::agent::adapter::constants::BINDING_SCHEMA_VERSION,
        terminal_id: target.terminal_id,
        kind: AgentKind::Codex,
        provider_session_id: AGENT_SESSION.into(),
        executable: EXECUTABLE.into(),
        cwd: CWD.into(),
        evidence_path: EVIDENCE_PATH.into(),
        data_home: DATA_HOME.into(),
        process_pid: Some(PROCESS_PID),
        process_start_time: Some(PROCESS_START),
        state,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: TIMESTAMP,
        updated_at_ms: TIMESTAMP,
    };
    let witness = NativeControlWitness::new(NativeControlWitnessFields {
        terminal_id: target.terminal_id,
        runtime_epoch: target.runtime_epoch,
        worker_id: Uuid::new_v4(),
        boot_id: BOOT_ID.into(),
        kind: binding.kind.clone(),
        process_pid: PROCESS_PID,
        process_start_time: PROCESS_START,
        binding_digest: binding_digest(&binding).unwrap(),
        authority_digest: NativeControlDigest::capture(&COMMAND).unwrap(),
    })
    .unwrap();
    NativeControlObservation::new(binding, witness, false).unwrap()
}

pub(super) fn interrupt_request() -> NativeInterruptRequest {
    let target = target();
    NativeInterruptRequest {
        terminal_id: target.terminal_id,
        runtime_epoch: target.runtime_epoch,
        request_id: Uuid::new_v4(),
        generation: GENERATION,
        writer_capability: WriterCapability::generate(),
        witness: observation(target, AgentState::Running).witness().clone(),
        command_digest: NativeControlDigest::capture(&COMMAND).unwrap(),
    }
}

pub(super) fn result(
    request: &NativeInterruptRequest,
    outcome: NativeControlOutcome,
) -> NativeControlResult {
    NativeControlResult {
        terminal_id: request.terminal_id,
        runtime_epoch: request.runtime_epoch,
        request_id: request.request_id,
        command_digest: request.command_digest,
        outcome,
    }
}

pub(super) const RESULT_MUTATIONS: &[fn(&mut NativeControlResult)] = &[
    |result| result.terminal_id = Uuid::new_v4(),
    |result| result.runtime_epoch = Uuid::new_v4(),
    |result| result.request_id = Uuid::new_v4(),
    |result| result.command_digest = NativeControlDigest::capture(&OTHER_COMMAND).unwrap(),
];

pub(super) fn assert_mismatch<T>(result: Result<T, ClientError>) {
    assert!(matches!(
        result,
        Err(ClientError::Protocol(ProtocolError::ResponseMismatch))
    ));
}
