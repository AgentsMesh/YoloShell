use uuid::Uuid;
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentTarget, ExecutionId, HubId, RuntimeEpoch, RuntimeTarget,
    TerminalAddress, TerminalId,
};
use yolo_hub_terminal_protocol::*;

#[path = "constants.rs"]
pub mod constants;
use constants::*;

pub fn address() -> TerminalAddress {
    TerminalAddress {
        hub_id: HubId::from_uuid(Uuid::from_u128(HUB_ID)).unwrap(),
        terminal_id: TerminalId::from_uuid(Uuid::from_u128(SESSION_ID)).unwrap(),
    }
}

pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: address(),
        runtime_epoch: RuntimeEpoch::from_uuid(Uuid::from_u128(RUNTIME_ID)).unwrap(),
    }
}

pub fn create_fields() -> CreateShellFields {
    CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::from_u128(REQUEST_ID),
        creation_token: Uuid::from_u128(CREATION_ID),
        name: Some(NAME.into()),
        cwd: DIRECTORY.into(),
        size: TerminalSize::default(),
    }
}

pub fn run_fields() -> TerminalRunFields {
    TerminalRunFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::from_u128(REQUEST_ID),
        creation_token: Uuid::from_u128(CREATION_ID),
        name: Some(NAME.into()),
        cwd: DIRECTORY.into(),
        size: TerminalSize::default(),
        input: b"secret-launch-command\r".to_vec(),
        workspace_id: None,
    }
}

pub fn summary_fields() -> TerminalSummaryFields {
    TerminalSummaryFields {
        presentation: Default::default(),
        address: address(),
        name: NAME.into(),
        cwd: DIRECTORY.into(),
        archived: false,
        recovery: None,
        runtime: Some(RuntimeSummary {
            target: target(),
            status: RuntimeStatus::Running,
            output_sequence: TEST_SEQUENCE,
            size: Some(TerminalSize::default()),
            safe_termination: None,
        }),
        agent: Some(AgentTarget {
            address: AgentAddress {
                hub_id: address().hub_id,
                agent_id: AgentId::from_uuid(Uuid::from_u128(AGENT_ID)).unwrap(),
            },
            execution_id: ExecutionId::from_uuid(Uuid::from_u128(RUNTIME_ID)).unwrap(),
        }),
        workspace: None,
    }
}

pub fn principal() -> WriterPrincipal {
    WriterPrincipal {
        client_id: Uuid::from_u128(CLIENT_ID),
        controller_instance_id: Uuid::from_u128(CONTROLLER_ID),
    }
}

pub fn grant_fields() -> LeaseGrantFields {
    LeaseGrantFields {
        target: target(),
        principal: principal(),
        generation: FIRST_LEASE_GENERATION,
        capability: SecretCapability::try_from(CAPABILITY_ID.to_owned()).unwrap(),
        expires_at_ms: LEASE_EXPIRY_MS,
        ttl_ms: LEASE_TTL_MS,
        renew_after_ms: LEASE_RENEW_MS,
    }
}

pub fn input_fields() -> InputFields {
    InputFields {
        request_id: Uuid::from_u128(REQUEST_ID),
        grant: LeaseGrant::new(grant_fields()).unwrap(),
        data: INPUT.to_vec(),
    }
}

pub fn snapshot_fields(sequence: u64) -> TerminalSnapshotFields {
    TerminalSnapshotFields {
        target: target(),
        barrier_sequence: sequence,
        size: TerminalSize::default(),
        data: OUTPUT.to_vec(),
    }
}

pub fn chunk_fields(sequence: u64) -> OutputChunkFields {
    OutputChunkFields {
        target: target(),
        sequence,
        data: OUTPUT.to_vec(),
    }
}

pub fn history_fields(sequence: u64) -> TerminalHistoryFields {
    TerminalHistoryFields {
        target: target(),
        barrier_sequence: sequence,
        size: TerminalSize::default(),
        data: OUTPUT.to_vec(),
    }
}

pub fn history_result_fields() -> AttachResultFields {
    AttachResultFields {
        target: target(),
        snapshot: None,
        chunks: Vec::new(),
        history: Some(TerminalHistory::new(history_fields(TEST_SEQUENCE)).unwrap()),
        next_sequence: TEST_SEQUENCE + STEP,
        resynchronized: false,
    }
}

pub fn replay_fields() -> AttachResultFields {
    AttachResultFields {
        target: target(),
        snapshot: None,
        chunks: vec![OutputChunk::new(chunk_fields(TEST_SEQUENCE)).unwrap()],
        history: None,
        next_sequence: TEST_SEQUENCE + STEP,
        resynchronized: false,
    }
}

pub fn snapshot_result_fields() -> AttachResultFields {
    AttachResultFields {
        target: target(),
        snapshot: Some(TerminalSnapshot::new(snapshot_fields(TEST_SEQUENCE)).unwrap()),
        chunks: Vec::new(),
        history: None,
        next_sequence: TEST_SEQUENCE + STEP,
        resynchronized: true,
    }
}

pub fn roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let encoded = serde_json::to_vec(value).unwrap();
    assert_eq!(&serde_json::from_slice::<T>(&encoded).unwrap(), value);
}

pub fn reject_field<T>(value: &T, field: &str, replacement: serde_json::Value)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut wire = serde_json::to_value(value).unwrap();
    wire[field] = replacement;
    assert!(serde_json::from_value::<T>(wire).is_err());
}
