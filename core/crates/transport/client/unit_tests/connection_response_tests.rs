use super::*;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method, to_value};
use tokio::net::UnixListener;
use tokio::sync::oneshot;

const REQUEST_ID: &str = "response-fence-request";
const METHOD: &str = method::LIST_SESSIONS;
const POISON_SOCKET_NAME: &str = "poison.sock";
const DECODE_POISON_SOCKET_NAME: &str = "decode-poison.sock";
const CANCELLATION_SOCKET_NAME: &str = "cancelled-response.sock";

#[test]
fn response_envelope_must_match_every_request_capability_field() {
    let request = RequestEnvelope::new(REQUEST_ID, METHOD, to_value(&()).unwrap());
    let valid = ResponseEnvelope::empty(&request);
    assert!(validate_response(&request, &valid).is_ok());

    let mut wrong_version = valid.clone();
    wrong_version.version = wrong_version.version.wrapping_add(1);
    let mut wrong_kind = valid.clone();
    wrong_kind.kind.push_str("-wrong");
    let mut wrong_identifier = valid.clone();
    wrong_identifier.request_id.push_str("-wrong");
    let mut wrong_method = valid.clone();
    wrong_method.method.push_str("-wrong");
    let mut success_with_error = valid.clone();
    success_with_error.error = Some(crate::protocol::ErrorBody::internal("unexpected"));
    let mut failure_with_payload =
        ResponseEnvelope::failure(&request, crate::protocol::ErrorBody::internal("failure"));
    failure_with_payload.payload = yolo_capnp_value::Value::from(1);
    let mut failure_without_error = failure_with_payload.clone();
    failure_without_error.payload = yolo_capnp_value::Value::Null;
    failure_without_error.error = None;
    for response in [
        wrong_version,
        wrong_kind,
        wrong_identifier,
        wrong_method,
        success_with_error,
        failure_with_payload,
        failure_without_error,
    ] {
        assert!(matches!(
            validate_response(&request, &response),
            Err(ProtocolError::ResponseMismatch)
        ));
    }
}

#[tokio::test]
async fn mismatched_response_poisons_connection_before_second_request() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(POISON_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        let request = stream.request().await.unwrap();
        let mut response = ResponseEnvelope::empty(&request);
        response.request_id.push_str("-wrong");
        assert!(matches!(
            stream.respond(&response).await,
            Err(ProtocolError::EndOfStream)
        ));
        stream.request().await
    });
    let client = Client::new(socket);
    let mut connection = client.connect().await.unwrap();
    assert!(matches!(
        connection.raw(METHOD, &()).await,
        Err(ClientError::Protocol(ProtocolError::ResponseMismatch))
    ));
    assert!(matches!(
        connection.raw(METHOD, &()).await,
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
    assert!(matches!(
        server.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}

#[tokio::test]
async fn undecodable_success_payload_poisons_connection_before_reuse() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(DECODE_POISON_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        let request = stream.request().await.unwrap();
        let response = ResponseEnvelope::success(&request, &()).unwrap();
        stream.respond(&response).await.unwrap();
        stream.request().await
    });
    let client = Client::new(socket);
    let mut connection = client.connect().await.unwrap();
    let result: Result<crate::model::HostStatus, ClientError> =
        connection.request(METHOD, &()).await;
    assert!(matches!(
        result,
        Err(ClientError::Protocol(ProtocolError::Decode(_)))
    ));
    assert!(matches!(
        connection.raw(METHOD, &()).await,
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
    assert!(matches!(
        server.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}

#[tokio::test]
async fn cancelling_a_pending_response_fails_closed_before_stream_reuse() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(CANCELLATION_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (partial_sent, partial_received) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        stream.request().await.unwrap();
        partial_sent.send(()).unwrap();
        stream.request().await
    });
    let client = Client::new(socket);
    let mut connection = client.connect().await.unwrap();
    {
        let response = connection.raw(METHOD, &());
        tokio::pin!(response);
        tokio::select! {
            biased;
            result = &mut response => panic!("partial response completed unexpectedly: {result:?}"),
            result = partial_received => result.unwrap(),
        }
    }
    assert!(matches!(
        connection.into_channel(),
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
    assert!(matches!(
        server.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}
