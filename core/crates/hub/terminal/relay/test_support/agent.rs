use super::{constants::*, fixtures::*};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    CommandOutcome, CommandReceipt, FIRST_OBSERVATION_SEQUENCE, ObservationCursor,
};
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId};
use yolo_hub_relay_control_protocol::{Request, RequestFields, SCHEMA_VERSION};

pub fn launch() -> Uuid {
    Uuid::from_u128(LAUNCH_VALUE)
}

pub fn interrupt() -> Request {
    let agent = AgentTarget {
        address: AgentAddress {
            hub_id: origin(),
            agent_id: AgentId::from_uuid(Uuid::from_u128(AGENT_VALUE)).unwrap(),
        },
        execution_id: ExecutionId::from_uuid(Uuid::from_u128(EXECUTION_VALUE)).unwrap(),
    };
    Request::new(RequestFields {
        schema_version: SCHEMA_VERSION,
        launch_id: launch(),
        terminal: target(),
        request: AgentControlRequest::new(AgentControlFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::from_u128(REQUEST_VALUE),
            target: agent,
            expected_cursor: ObservationCursor::new(
                agent,
                Uuid::from_u128(STREAM_VALUE),
                FIRST_OBSERVATION_SEQUENCE,
            )
            .unwrap(),
            action: AgentAction::Interrupt {},
        })
        .unwrap(),
    })
    .unwrap()
}

pub fn delivered(request: &Request) -> CommandReceipt {
    let fields = request.fields().request.fields();
    let mut receipt = CommandReceipt::accepted(fields.request_id, fields.target).unwrap();
    receipt.advance(CommandOutcome::Delivered {}).unwrap();
    receipt
}
