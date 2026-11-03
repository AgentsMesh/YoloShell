use super::{
    agent_control_fixture::*,
    fixtures::{constants::*, *},
    server::*,
};
use crate::ClientError;
use tokio::time::Instant;
use yolo_hub_agent_protocol::FailureCode;
use yolo_hub_identity::{AgentId, ExecutionId};
use yolo_hub_jsonrpc_protocol::Call as Request;
use yolo_hub_jsonrpc_transport::{Incoming, TransportError};
use yolo_hub_rpc_protocol::Reply;

#[tokio::test]
async fn mismatched_receipt_identity_after_dispatch_is_unknown_and_cannot_be_replayed() {
    for defect in [Defect::Request, Defect::Agent, Defect::Execution] {
        let (mut client, mut server) = connected().await;
        let mut target = agent_target();
        let request_id = match defect {
            Defect::Request => id(OTHER_REQUEST_ID),
            Defect::Agent => {
                target.address.agent_id = AgentId::from_uuid(id(OTHER_AGENT_ID)).unwrap();
                id(REQUEST_ID)
            }
            Defect::Execution => {
                target.execution_id = ExecutionId::from_uuid(id(OTHER_EXECUTION_ID)).unwrap();
                id(REQUEST_ID)
            }
        };
        let peer = async {
            let request = receive(&mut server).await;
            assert_eq!(request.fields().command, command());
            respond(
                &mut server,
                &request,
                Reply::AgentControl {
                    receipt: rejected(request_id, target, FailureCode::Unsupported),
                },
            )
            .await;
            assert_eq!(
                server.receive::<Request>(deadline()).await.unwrap(),
                Incoming::Eof
            );
        };
        let caller = async {
            let error = client.call(hub(), command(), deadline()).await.unwrap_err();
            assert!(matches!(error, ClientError::OutcomeUnknownResponse(_)));
            assert!(error.outcome_is_unknown());
            assert!(!client.is_usable());
            assert!(matches!(
                client.call(hub(), command(), deadline()).await,
                Err(ClientError::Poisoned)
            ));
        };
        tokio::join!(caller, peer);
    }
}

#[tokio::test]
async fn dropped_semantic_response_is_unknown_without_automatic_retry() {
    let (mut client, mut server) = connected().await;
    let peer = async move {
        let request = receive(&mut server).await;
        assert_eq!(request.fields().command, command());
    };
    let (result, ()) = tokio::join!(client.call(hub(), command(), deadline()), peer);
    assert!(matches!(
        result,
        Err(ClientError::OutcomeUnknown(TransportError::TruncatedFrame))
    ));
    assert!(!client.is_usable());
    assert!(matches!(
        client.call(hub(), command(), deadline()).await,
        Err(ClientError::Poisoned)
    ));
}

#[tokio::test]
async fn pending_semantic_response_deadline_retires_channel_and_does_not_repeat_request() {
    let (mut client, mut server) = connected().await;
    let peer = async {
        let request = receive(&mut server).await;
        assert_eq!(request.fields().command, command());
        assert_eq!(
            server.receive::<Request>(deadline()).await.unwrap(),
            Incoming::Eof
        );
    };
    let caller = async {
        let result = client
            .call(hub(), command(), Instant::now() + SHORT_TIMEOUT)
            .await;
        assert!(matches!(
            result,
            Err(ClientError::OutcomeUnknown(TransportError::Deadline))
        ));
        assert!(!client.is_usable());
        assert!(matches!(
            client.call(hub(), command(), deadline()).await,
            Err(ClientError::Poisoned)
        ));
    };
    tokio::join!(caller, peer);
}

enum Defect {
    Request,
    Agent,
    Execution,
}
