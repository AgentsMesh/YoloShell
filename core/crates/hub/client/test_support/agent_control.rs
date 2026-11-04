use super::fixtures::{constants::*, *};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    CommandOutcome, CommandReceipt, FIRST_OBSERVATION_SEQUENCE, FailureCode, ObservationCursor,
};
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId};
use yolo_hub_rpc_protocol::Command;

pub fn agent_target() -> AgentTarget {
    AgentTarget {
        address: AgentAddress {
            hub_id: hub(),
            agent_id: AgentId::from_uuid(id(AGENT_ID)).unwrap(),
        },
        execution_id: ExecutionId::from_uuid(id(EXECUTION_ID)).unwrap(),
    }
}

pub fn command() -> Command {
    let target = agent_target();
    let request = AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: id(REQUEST_ID),
        target,
        expected_cursor: ObservationCursor::new(
            target,
            id(OBSERVATION_STREAM_ID),
            FIRST_OBSERVATION_SEQUENCE,
        )
        .unwrap(),
        action: AgentAction::Interrupt {},
    })
    .unwrap();
    Command::AgentControl {
        request,
        grant: grant(),
    }
}

pub fn rejected(request_id: uuid::Uuid, target: AgentTarget, code: FailureCode) -> CommandReceipt {
    let mut receipt = CommandReceipt::accepted(request_id, target).unwrap();
    receipt.advance(CommandOutcome::Failed { code }).unwrap();
    receipt
}
