use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_agent_protocol::{Activity, AgentWorkingDirectory, Lifecycle, ObservationCursor};
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::*;

#[path = "constants.rs"]
pub mod constants;
use constants::*;

pub fn hub() -> HubId {
    HubId::from_uuid(Uuid::from_u128(HUB_ID)).unwrap()
}
pub fn other_hub() -> HubId {
    HubId::from_uuid(Uuid::from_u128(OTHER_ID)).unwrap()
}
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
pub fn connector() -> ConnectorId {
    ConnectorId::from_uuid(Uuid::from_u128(CONNECTOR_ID)).unwrap()
}
pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: TerminalAddress {
            hub_id: hub(),
            terminal_id: TerminalId::from_uuid(Uuid::from_u128(SESSION_ID)).unwrap(),
        },
        runtime_epoch: RuntimeEpoch::from_uuid(Uuid::from_u128(RUNTIME_ID)).unwrap(),
    }
}
pub fn agent() -> AgentTarget {
    AgentTarget {
        address: AgentAddress {
            hub_id: hub(),
            agent_id: AgentId::from_uuid(Uuid::from_u128(AGENT_ID)).unwrap(),
        },
        execution_id: ExecutionId::from_uuid(Uuid::from_u128(EXECUTION_ID)).unwrap(),
    }
}
pub fn request(command: Command) -> Request {
    Request {
        version: CONNECTOR_PROTOCOL_VERSION,
        request_id: Uuid::from_u128(REQUEST_ID),
        hub: hub(),
        command,
    }
}
pub fn hello() -> Request {
    request(Command::Hello {})
}
pub fn activation_request() -> Request {
    request(Command::Activate {
        connector_id: connector(),
        credential: ConnectorCredential::try_from(CREDENTIAL.to_owned()).unwrap(),
        instance_id: ConnectorInstanceId::from_uuid(Uuid::from_u128(INSTANCE_ID)).unwrap(),
    })
}
pub fn activation() -> Activation {
    Activation {
        connector_id: connector(),
        generation: ConnectorGeneration::from_uuid(Uuid::from_u128(GENERATION_ID)).unwrap(),
        agent: agent(),
        terminal: target(),
        next_sequence: FIRST_PUBLICATION_SEQUENCE,
    }
}
pub fn publication_request() -> Request {
    request(Command::Publish {
        publication: Publication {
            connector_id: connector(),
            generation: activation().generation,
            agent: agent(),
            terminal: target(),
            sequence: FIRST_PUBLICATION_SEQUENCE,
            state: ReportedState {
                lifecycle: Lifecycle::Running {},
                activity: Activity::Working,
                working_directory: Some(AgentWorkingDirectory::new(DIRECTORY.into()).unwrap()),
            },
        },
    })
}
pub fn acknowledgement(duplicate: bool) -> PublicationAck {
    PublicationAck {
        connector_id: connector(),
        generation: activation().generation,
        sequence: FIRST_PUBLICATION_SEQUENCE,
        cursor: ObservationCursor::new(
            agent(),
            Uuid::from_u128(OBSERVATION_STREAM),
            OBSERVATION_SEQUENCE,
        )
        .unwrap(),
        duplicate,
    }
}
