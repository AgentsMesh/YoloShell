use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::ClientError;
use yolo_hub_connector_jsonrpc::Answer;
use yolo_hub_connector_protocol::{ConnectorError, Reply};
use yolo_hub_identity::{ConnectorGeneration, ExecutionId};

#[tokio::test]
async fn activation_and_publication_preserve_exact_scope_and_duplicate_acknowledgement() {
    for duplicate in [false, true] {
        let (mut client, mut server) = connected().await;
        let peer = async {
            let request = receive(&mut server).await;
            assert_eq!(request, activation_request());
            respond(
                &mut server,
                &request,
                Reply::Activated {
                    activation: activation(),
                },
            )
            .await;
            let request = receive(&mut server).await;
            assert_eq!(request, publication_request());
            respond(
                &mut server,
                &request,
                Reply::Published {
                    ack: acknowledgement(duplicate),
                },
            )
            .await;
        };
        let caller = async {
            assert_eq!(
                client.call(activation_request(), deadline()).await.unwrap(),
                Reply::Activated {
                    activation: activation()
                }
            );
            assert_eq!(
                client
                    .call(publication_request(), deadline())
                    .await
                    .unwrap(),
                Reply::Published {
                    ack: acknowledgement(duplicate)
                }
            );
            assert!(client.is_usable());
        };
        tokio::join!(caller, peer);
    }
}

#[tokio::test]
async fn definitive_rejections_preserve_channel_and_never_repeat_the_rejected_command() {
    for code in [
        ConnectorError::Unauthorized,
        ConnectorError::StaleGeneration,
        ConnectorError::OutcomeUnknown,
    ] {
        let (mut client, mut server) = connected().await;
        let peer = async {
            let request = receive(&mut server).await;
            assert_eq!(request, publication_request());
            respond(&mut server, &request, Reply::Error { code }).await;
            let request = receive(&mut server).await;
            assert_eq!(request, hello());
            respond(&mut server, &request, Reply::Hello { hub: hub() }).await;
        };
        let caller = async {
            let error = client
                .call(publication_request(), deadline())
                .await
                .unwrap_err();
            assert!(matches!(error, ClientError::Remote(found) if found == code));
            assert_eq!(
                error.outcome_is_unknown(),
                code == ConnectorError::OutcomeUnknown
            );
            assert!(client.is_usable());
            client.call(hello(), deadline()).await.unwrap();
        };
        tokio::join!(caller, peer);
    }
}

#[tokio::test]
async fn wrong_correlation_generation_or_execution_after_publication_is_unknown() {
    for defect in [Defect::Request, Defect::Generation, Defect::Execution] {
        let (mut client, mut server) = connected().await;
        let peer = async {
            let request = receive(&mut server).await;
            let mut ack = acknowledgement(false);
            match defect {
                Defect::Generation => ack.generation = ConnectorGeneration::new(),
                Defect::Execution => {
                    let mut agent = agent();
                    agent.execution_id = ExecutionId::new();
                    ack.cursor = yolo_hub_agent_protocol::ObservationCursor::new(
                        agent,
                        ack.cursor.stream(),
                        ack.cursor.sequence(),
                    )
                    .unwrap();
                }
                Defect::Request => {}
            }
            let mut response = Answer::new(&request, Reply::Published { ack }).unwrap();
            if matches!(defect, Defect::Request) {
                response.0.id = Some(yolo_hub_jsonrpc_protocol::Id::fresh());
            }
            server.send(&response, deadline()).await.unwrap();
            eof(&mut server).await;
        };
        let caller = async {
            let error = client
                .call(publication_request(), deadline())
                .await
                .unwrap_err();
            assert!(matches!(error, ClientError::OutcomeUnknownResponse(_)));
            assert!(!client.is_usable());
            assert!(matches!(
                client.call(publication_request(), deadline()).await,
                Err(ClientError::Poisoned)
            ));
        };
        tokio::join!(caller, peer);
    }
}

enum Defect {
    Request,
    Generation,
    Execution,
}
