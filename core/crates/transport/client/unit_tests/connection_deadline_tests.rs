use super::*;
use crate::protocol::{ProtocolError, RequestEnvelope, method};
use std::future::pending;
use std::time::Duration;
use tokio::net::UnixListener;
use tokio::sync::oneshot;

const RESPONSE_DEADLINE: Duration = Duration::from_secs(1);

#[tokio::test(start_paused = true)]
async fn stalled_response_exhausts_one_deadline_and_poisons_the_connection() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("deadline.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let (accepted_tx, accepted_rx) = oneshot::channel();
    let (request_tx, request_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        accepted_tx.send(()).unwrap();
        let request: RequestEnvelope = stream.request().await.unwrap();
        request_tx.send(request).unwrap();
        pending::<()>().await;
    });
    let client = Client::new(socket);
    let connection = tokio::spawn(async move { client.connect().await.unwrap() });
    accepted_rx.await.unwrap();
    let mut connection = connection.await.unwrap();
    let deadline = ClientDeadline::test_after(RESPONSE_DEADLINE);
    let request = tokio::spawn(async move {
        let result = connection
            .raw_until(method::HOST_STATUS, &(), deadline)
            .await;
        (connection, result)
    });
    let observed = crate::rpc_test_support::scheduled(request_rx).await;
    assert_eq!(observed.method, method::HOST_STATUS);

    tokio::time::advance(RESPONSE_DEADLINE).await;
    let (mut connection, result) = request.await.unwrap();

    assert!(matches!(
        result,
        Err(ClientError::Protocol(ProtocolError::OutcomeUnknown))
    ));
    assert!(matches!(
        connection.raw(method::HOST_STATUS, &()).await,
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
    server.abort();
    let _ = server.await;
}
