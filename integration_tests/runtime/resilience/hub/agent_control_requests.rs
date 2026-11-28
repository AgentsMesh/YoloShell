use super::super::federation_support::{RunningNode, deadline};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    CommandOutcome, FailureCode, ObservationCursor,
};
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError};

pub fn request(cursor: ObservationCursor, action: AgentAction) -> AgentControlRequest {
    AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        target: cursor.target(),
        expected_cursor: cursor,
        action,
    })
    .unwrap()
}

pub async fn receipt(
    client: &mut LocalClient,
    request: AgentControlRequest,
    grant: LeaseGrant,
    code: FailureCode,
) {
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
    assert_eq!(receipt.outcome(), CommandOutcome::Failed { code });
    assert!(client.is_usable());
}

pub async fn denied(node: &RunningNode, cursor: ObservationCursor, grant: LeaseGrant) {
    let mut client = node.client().await;
    let command = Command::AgentControl {
        request: request(cursor, AgentAction::Interrupt {}),
        grant,
    };
    let error = client
        .call(cursor.target().address.hub_id, command, deadline())
        .await
        .unwrap_err();
    assert!(
        matches!(error, ClientError::Remote(PublicError::Unreachable)),
        "{error:?}"
    );
    assert!(!error.outcome_is_unknown());
    assert!(client.is_usable());
}
