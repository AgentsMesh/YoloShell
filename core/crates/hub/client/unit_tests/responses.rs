use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::ClientError;
use yolo_hub_jsonrpc_protocol::Answer as Response;
use yolo_hub_jsonrpc_transport::Incoming;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn mismatched_responses_poison_and_mutations_explicitly_report_unknown_outcome() {
    #[derive(Clone, Copy)]
    enum Defect {
        Version,
        Correlation,
        BothOutcomes,
        Payload,
    }
    for defect in [
        Defect::Version,
        Defect::Correlation,
        Defect::BothOutcomes,
        Defect::Payload,
    ] {
        for mutation in [false, true] {
            let (mut client, mut server) = connected().await;
            let command = if mutation { input() } else { list() };
            let peer = async {
                let request = receive(&mut server).await;
                let reply = if mutation {
                    Reply::Input { duplicate: false }
                } else {
                    Reply::Terminals {
                        terminals: Vec::new(),
                        next: None,
                    }
                };
                let mut response = Response::new(&request, reply).unwrap();
                match defect {
                    Defect::Version => response.jsonrpc = String::new(),
                    Defect::Correlation => {
                        response.id = Some(yolo_hub_jsonrpc_protocol::Id::fresh())
                    }
                    Defect::BothOutcomes => {
                        response.error = Response::new(
                            &request,
                            Reply::Error {
                                code: PublicError::Unauthorized,
                            },
                        )
                        .unwrap()
                        .error
                    }
                    Defect::Payload => {
                        response.result =
                            Response::new(&request, Reply::Completed {}).unwrap().result
                    }
                }
                server.send(&response, deadline()).await.unwrap();
                assert_eq!(
                    server
                        .receive::<yolo_hub_jsonrpc_protocol::Call>(deadline())
                        .await
                        .unwrap(),
                    Incoming::Eof
                );
            };
            let caller = async {
                let error = client.call(hub(), command, deadline()).await.err().unwrap();
                assert_eq!(error.outcome_is_unknown(), mutation);
                if mutation {
                    assert!(matches!(error, ClientError::OutcomeUnknownResponse(_)));
                } else {
                    assert!(matches!(error, ClientError::InvalidResponse(_)));
                }
                assert!(!client.is_usable());
                assert!(matches!(
                    client.call(hub(), input(), deadline()).await,
                    Err(ClientError::Poisoned)
                ));
            };
            tokio::join!(caller, peer);
        }
    }
}

#[tokio::test]
async fn confirmed_remote_error_retains_channel_and_never_retries_the_operation() {
    for code in [PublicError::Unauthorized, PublicError::OutcomeUnknown] {
        let (mut client, mut server) = connected().await;
        let peer = async {
            let first = receive(&mut server).await;
            assert_eq!(first.fields().command, input());
            assert_eq!(first.fields().request_id, id(REQUEST_ID));
            respond(&mut server, &first, Reply::Error { code }).await;
            let second = receive(&mut server).await;
            assert_eq!(second.fields().command, list());
            assert_ne!(second.fields().request_id, first.fields().request_id);
            respond(
                &mut server,
                &second,
                Reply::Terminals {
                    terminals: Vec::new(),
                    next: None,
                },
            )
            .await;
        };
        let caller = async {
            let error = client.call(hub(), input(), deadline()).await.err().unwrap();
            assert!(matches!(error, ClientError::Remote(found) if found == code));
            assert_eq!(
                error.outcome_is_unknown(),
                code == PublicError::OutcomeUnknown
            );
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
async fn successful_input_preserves_delivery_identity_and_duplicate_flag() {
    for duplicate in [false, true] {
        let (mut client, mut server) = connected().await;
        let peer = async {
            let request = receive(&mut server).await;
            assert_eq!(request.fields().request_id, id(REQUEST_ID));
            assert_eq!(request.fields().command, input());
            respond(&mut server, &request, Reply::Input { duplicate }).await;
        };
        let (reply, ()) = tokio::join!(client.call(hub(), input(), deadline()), peer);
        assert!(matches!(reply, Ok(Reply::Input { duplicate: found }) if found == duplicate));
        assert!(client.is_usable());
    }
}
