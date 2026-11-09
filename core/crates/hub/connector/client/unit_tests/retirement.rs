use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::ClientError;
use tokio::time::Instant;
use yolo_hub_jsonrpc_transport::TransportError;

#[tokio::test]
async fn dropped_reply_and_deadline_after_send_are_unknown_for_mutations_only() {
    for mutation in [false, true] {
        for dropped in [false, true] {
            let (mut client, mut server) = connected().await;
            let request = if mutation {
                activation_request()
            } else {
                hello()
            };
            let expected = request.clone();
            let peer = async move {
                assert_eq!(receive(&mut server).await, expected);
                if !dropped {
                    eof(&mut server).await;
                }
            };
            let caller = async {
                let error = client
                    .call(request, Instant::now() + SHORT_TIMEOUT)
                    .await
                    .unwrap_err();
                assert_eq!(error.outcome_is_unknown(), mutation);
                let cause = match error {
                    ClientError::OutcomeUnknown(cause) | ClientError::Transport(cause) => cause,
                    other => panic!("{other:?}"),
                };
                assert!(matches!(
                    (cause, dropped),
                    (TransportError::TruncatedFrame, true) | (TransportError::Deadline, false)
                ));
                assert!(!client.is_usable());
                assert!(matches!(
                    client.call(activation_request(), deadline()).await,
                    Err(ClientError::Poisoned)
                ));
            };
            tokio::join!(caller, peer);
        }
    }
}

#[tokio::test]
async fn cancellation_after_delivery_closes_channel_instead_of_resuming_partial_response() {
    let (mut client, mut server) = connected().await;
    let (sent, received) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        assert_eq!(receive(&mut server).await, publication_request());
        sent.send(()).unwrap();
        eof(&mut server).await;
    });
    let mut pending = Box::pin(client.call(publication_request(), deadline()));
    tokio::select! { result = &mut pending => panic!("{result:?}"), result = received => result.unwrap() }
    drop(pending);
    assert!(!client.is_usable());
    assert!(matches!(
        client.call(publication_request(), deadline()).await,
        Err(ClientError::Poisoned)
    ));
    peer.await.unwrap();
}

#[tokio::test]
async fn failed_write_retires_the_connection_without_guessing_mutation_delivery() {
    for mutation in [false, true] {
        let (mut client, server) = connected().await;
        drop(server);
        let request = if mutation {
            activation_request()
        } else {
            hello()
        };
        let error = client.call(request, deadline()).await.unwrap_err();
        assert_eq!(error.outcome_is_unknown(), mutation);
        assert!(matches!(
            error,
            ClientError::OutcomeUnknown(TransportError::Io(_))
                | ClientError::Transport(TransportError::Io(_))
        ));
        assert!(!client.is_usable());
    }
}
