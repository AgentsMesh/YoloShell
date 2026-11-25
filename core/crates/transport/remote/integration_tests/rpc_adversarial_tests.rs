use super::super::RemoteError;
use super::super::constants::{REMOTE_REQUEST_FAILED_MESSAGE, SSH_OPERATION_TIMEOUT};
use super::super::deadline::RemoteDeadline;
use super::super::rpc::RemoteRpc;
use crate::model::PROTOCOL_VERSION;
use crate::protocol::{ErrorBody, ProtocolError, ResponseEnvelope, method};
use std::sync::Arc;
use tokio::sync::Notify;

const FIRST_VALUE: u64 = 7;
const SECOND_VALUE: u64 = 11;
const INVALID_KIND: &str = "invalid-response-kind";
const INVALID_REQUEST_ID: &str = "invalid-request-id";
const INVALID_METHOD: &str = "invalid-method";
const INVALID_PAYLOAD: &str = "invalid-payload";
const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;

#[derive(Clone, Copy)]
enum Corruption {
    Version,
    Kind,
    RequestId,
    Method,
}

#[tokio::test]
async fn mismatched_response_fences_the_channel_before_another_write() {
    for corruption in [
        Corruption::Version,
        Corruption::Kind,
        Corruption::RequestId,
        Corruption::Method,
    ] {
        let (mut rpc, server) = corrupted_rpc(corruption);
        let error = rpc
            .request_until::<_, u64>(method::HOST_STATUS, &(), RemoteDeadline::operation())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            RemoteError::Protocol(ProtocolError::ResponseMismatch)
        ));
        assert_poisoned(&mut rpc).await;
        server.await.unwrap();
    }
}

#[tokio::test]
async fn payload_decode_failure_poisoning_does_not_apply_to_host_failures() {
    let (client, server) = tokio::io::duplex(super::support::TEST_CHANNEL_CAPACITY);
    let server = super::rpc_fixture::serve(server, |request| {
        Box::pin(async move {
            super::rpc_fixture::reply(
                ResponseEnvelope::success(&request, &INVALID_PAYLOAD).unwrap(),
            )
        })
    });
    let mut rpc = RemoteRpc::new(Box::new(client));
    let error = rpc
        .request_until::<_, u64>(method::HOST_STATUS, &(), RemoteDeadline::operation())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RemoteError::Protocol(ProtocolError::Decode(_))
    ));
    assert_poisoned(&mut rpc).await;
    server.await.unwrap();

    let (mut rpc, server) = host_failure_rpc();
    assert!(matches!(
        rpc.request_until::<_, u64>(method::HOST_STATUS, &(), RemoteDeadline::operation())
            .await,
        Err(RemoteError::Host(_))
    ));
    let value = rpc
        .request_until::<_, u64>(method::HOST_STATUS, &(), RemoteDeadline::operation())
        .await
        .unwrap();
    assert_eq!(value, SECOND_VALUE);
    drop(rpc);
    server.await.unwrap();
}

#[tokio::test]
async fn stalled_response_times_out_and_poisoning_closes_the_channel() {
    let reached = Arc::new(Notify::new());
    let (client, server) = tokio::io::duplex(super::support::TEST_CHANNEL_CAPACITY);
    let server_reached = reached.clone();
    let server = super::rpc_fixture::serve(server, move |_| {
        let reached = server_reached.clone();
        Box::pin(async move {
            reached.notify_one();
            std::future::pending().await
        })
    });
    let mut rpc = RemoteRpc::new(Box::new(client));
    let request = tokio::spawn(async move {
        let error = rpc
            .request_until::<_, u64>(method::HOST_STATUS, &(), RemoteDeadline::operation())
            .await
            .unwrap_err();
        (rpc, error)
    });
    reached.notified().await;
    tokio::time::pause();
    tokio::time::advance(SSH_OPERATION_TIMEOUT).await;
    let (mut rpc, error) = request.await.unwrap();
    assert!(matches!(
        error,
        RemoteError::Protocol(crate::protocol::ProtocolError::OutcomeUnknown)
    ));
    assert_poisoned(&mut rpc).await;
    server.await.unwrap();
}

fn corrupted_rpc(corruption: Corruption) -> (RemoteRpc, tokio::task::JoinHandle<()>) {
    let (client, server) = tokio::io::duplex(super::support::TEST_CHANNEL_CAPACITY);
    let task = super::rpc_fixture::serve(server, move |request| {
        Box::pin(async move {
            let mut response = ResponseEnvelope::success(&request, &FIRST_VALUE).unwrap();
            corrupt(&mut response, corruption);
            super::rpc_fixture::reply(response)
        })
    });
    (RemoteRpc::new(Box::new(client)), task)
}

fn host_failure_rpc() -> (RemoteRpc, tokio::task::JoinHandle<()>) {
    let (client, server) = tokio::io::duplex(super::support::TEST_CHANNEL_CAPACITY);
    let first = std::sync::atomic::AtomicBool::new(true);
    let task = super::rpc_fixture::serve(server, move |request| {
        let first = first.swap(false, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            super::rpc_fixture::reply(if first {
                ResponseEnvelope::failure(
                    &request,
                    ErrorBody::internal(REMOTE_REQUEST_FAILED_MESSAGE),
                )
            } else {
                ResponseEnvelope::success(&request, &SECOND_VALUE).unwrap()
            })
        })
    });
    (RemoteRpc::new(Box::new(client)), task)
}

fn corrupt(response: &mut ResponseEnvelope, corruption: Corruption) {
    match corruption {
        Corruption::Version => {
            response.version =
                PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);
        }
        Corruption::Kind => response.kind = INVALID_KIND.into(),
        Corruption::RequestId => response.request_id = INVALID_REQUEST_ID.into(),
        Corruption::Method => response.method = INVALID_METHOD.into(),
    }
}

async fn assert_poisoned(rpc: &mut RemoteRpc) {
    let error = rpc
        .request_until::<_, u64>(method::HOST_STATUS, &(), RemoteDeadline::operation())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RemoteError::Protocol(ProtocolError::ConnectionPoisoned)
    ));
}
