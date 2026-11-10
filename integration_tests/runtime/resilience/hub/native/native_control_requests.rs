use super::super::{federation_support::deadline, support::acquire};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    CommandOutcome, CommandReceipt, ObservationCursor,
};
use yolo_hub_client::LocalClient;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, LeaseMode};

pub fn interrupt(cursor: ObservationCursor) -> AgentControlRequest {
    AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        target: cursor.target(),
        expected_cursor: cursor,
        action: AgentAction::Interrupt {},
    })
    .unwrap()
}

pub async fn acquire_writer(
    client: &mut LocalClient,
    target: RuntimeTarget,
    mode: LeaseMode,
) -> LeaseGrant {
    let Reply::Lease { grant } = client
        .call(target.address.hub_id, acquire(target, mode), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    grant
}

pub async fn control(
    client: &mut LocalClient,
    request: AgentControlRequest,
    grant: LeaseGrant,
) -> CommandReceipt {
    let fields = request.fields().clone();
    let Reply::AgentControl { receipt } = client
        .call(
            fields.target.address.hub_id,
            Command::AgentControl { request, grant },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(receipt.request_id(), fields.request_id);
    assert_eq!(receipt.target(), fields.target);
    assert!(client.is_usable());
    receipt
}

pub fn not_delivered(receipt: CommandReceipt) {
    assert!(!matches!(
        receipt.outcome(),
        CommandOutcome::Delivered {} | CommandOutcome::Confirmed {} | CommandOutcome::Accepted {}
    ));
}
