use super::{agent, constants::REQUEST_ID, id};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    CommandOutcome, CommandReceipt, FailureCode,
};

pub fn agent_control() -> AgentControlRequest {
    let snapshot = agent();
    AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: id(REQUEST_ID),
        target: snapshot.cursor().target(),
        expected_cursor: snapshot.cursor(),
        action: AgentAction::Interrupt {},
    })
    .unwrap()
}

pub fn agent_receipt() -> CommandReceipt {
    let request = agent_control();
    let mut receipt =
        CommandReceipt::accepted(request.fields().request_id, request.fields().target).unwrap();
    receipt
        .advance(CommandOutcome::Failed {
            code: FailureCode::Unsupported,
        })
        .unwrap();
    receipt
}
