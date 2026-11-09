use super::{
    agent_control_fixture::*,
    fixtures::{constants::*, *},
    server::*,
};
use crate::ClientError;
use yolo_hub_agent_protocol::{CommandOutcome, FailureCode};
use yolo_hub_rpc_protocol::{Method, Reply};

#[tokio::test]
async fn exact_failed_semantic_receipt_is_returned_without_retry_or_channel_retirement() {
    for code in [
        FailureCode::Unsupported,
        FailureCode::PreconditionsChanged,
        FailureCode::PermissionDenied,
    ] {
        let (mut client, mut server) = connected().await;
        let expected = rejected(id(REQUEST_ID), agent_target(), code);
        let peer = async {
            let request = receive(&mut server).await;
            assert_eq!(request.fields().command, command());
            assert_eq!(request.fields().request_id, id(REQUEST_ID));
            respond(
                &mut server,
                &request,
                Reply::AgentControl { receipt: expected },
            )
            .await;
            let next = receive(&mut server).await;
            assert_eq!(next.fields().command, list());
            respond(
                &mut server,
                &next,
                Reply::Terminals {
                    terminals: vec![],
                    next: None,
                },
            )
            .await;
        };
        let caller = async {
            let reply = client.call(hub(), command(), deadline()).await.unwrap();
            assert_eq!(reply, Reply::AgentControl { receipt: expected });
            assert_eq!(expected.outcome(), CommandOutcome::Failed { code });
            assert!(client.is_usable());
            assert!(matches!(
                client.call(hub(), list(), deadline()).await,
                Ok(Reply::Terminals { .. })
            ));
        };
        tokio::join!(caller, peer);
    }
}

#[tokio::test]
async fn unadvertised_semantic_method_is_rejected_before_any_request_bytes_are_sent() {
    let (mut client, mut server) = connected_with(vec![Method::Hello, Method::List]).await;
    let error = client.call(hub(), command(), deadline()).await.unwrap_err();
    assert!(matches!(error, ClientError::Unsupported));
    assert!(!error.outcome_is_unknown());
    assert!(client.is_usable());
    let peer = async {
        let request = receive(&mut server).await;
        assert_eq!(request.fields().command, list());
        respond(
            &mut server,
            &request,
            Reply::Terminals {
                terminals: vec![],
                next: None,
            },
        )
        .await;
    };
    let (reply, ()) = tokio::join!(client.call(hub(), list(), deadline()), peer);
    assert!(matches!(reply, Ok(Reply::Terminals { .. })));
}
