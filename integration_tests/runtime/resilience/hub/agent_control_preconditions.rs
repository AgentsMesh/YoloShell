use super::{deadline, requests};
use yolo_hub_agent_protocol::{
    AgentAction, FIRST_OBSERVATION_SEQUENCE, FailureCode, ObservationCursor,
};
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_identity::{AgentId, AgentTarget, ExecutionId, HubId, RuntimeEpoch};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError};

pub async fn verify(
    client: &mut LocalClient,
    cursor: ObservationCursor,
    grant: LeaseGrant,
    foreign: HubId,
) {
    let different_execution = AgentTarget {
        execution_id: ExecutionId::new(),
        ..cursor.target()
    };
    let changed_execution =
        ObservationCursor::new(different_execution, cursor.stream(), cursor.sequence()).unwrap();
    let changed_stream =
        ObservationCursor::new(cursor.target(), uuid::Uuid::new_v4(), cursor.sequence()).unwrap();
    let changed_sequence = ObservationCursor::new(
        cursor.target(),
        cursor.stream(),
        cursor
            .sequence()
            .checked_add(FIRST_OBSERVATION_SEQUENCE)
            .unwrap(),
    )
    .unwrap();
    for stale in [changed_execution, changed_stream, changed_sequence] {
        requests::receipt(
            client,
            requests::request(stale, AgentAction::Interrupt {}),
            grant,
            FailureCode::PreconditionsChanged,
        )
        .await;
    }
    // These altered grants are malformed request metadata, not authenticated tokens.
    let mut wrong_runtime = *grant.fields();
    wrong_runtime.target.runtime_epoch = RuntimeEpoch::new();
    requests::receipt(
        client,
        requests::request(cursor, AgentAction::Interrupt {}),
        LeaseGrant::new(wrong_runtime).unwrap(),
        FailureCode::PreconditionsChanged,
    )
    .await;
    let mut foreign_grant = *grant.fields();
    foreign_grant.target.address.hub_id = foreign;
    let error = client
        .call(
            cursor.target().address.hub_id,
            Command::AgentControl {
                request: requests::request(cursor, AgentAction::Interrupt {}),
                grant: LeaseGrant::new(foreign_grant).unwrap(),
            },
            deadline(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Validation(PublicError::OriginMismatch)
    ));
    assert!(!error.outcome_is_unknown());
    assert!(client.is_usable());
    let mut unknown = cursor.target();
    unknown.address.agent_id = AgentId::new();
    let unknown = ObservationCursor::new(unknown, cursor.stream(), cursor.sequence()).unwrap();
    let error = client
        .call(
            cursor.target().address.hub_id,
            Command::AgentControl {
                request: requests::request(unknown, AgentAction::Interrupt {}),
                grant,
            },
            deadline(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, ClientError::Remote(PublicError::NotFound)),
        "{error:?}"
    );
    assert!(!error.outcome_is_unknown());
    assert!(client.is_usable());
}
