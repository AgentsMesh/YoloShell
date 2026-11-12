use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::{ClientError, exchange::send};
use serde::{Serialize, Serializer};
use tokio::{io::duplex, time::Instant};
use yolo_hub_jsonrpc_transport::{Framed, Incoming, MAX_FRAME_BYTES, TransportError};
use yolo_hub_rpc_protocol::Reply;

struct EncodingFailure;
impl Serialize for EncodingFailure {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(SERIALIZATION_FAILURE))
    }
}

#[tokio::test]
async fn encoding_failure_is_known_not_delivered_and_preserves_the_channel() {
    let (left, right) = duplex(PIPE_CAPACITY);
    let mut channel = Framed::new(left);
    let mut server = Framed::new(right);
    let failure = send(&mut channel, &EncodingFailure, true, deadline())
        .await
        .err()
        .unwrap();
    assert!(matches!(
        failure.error,
        ClientError::Transport(TransportError::Encode(_))
    ));
    assert!(!failure.error.outcome_is_unknown());
    assert!(failure.reusable && channel.is_usable());
    send(&mut channel, &SAMPLE_PAYLOAD, true, deadline())
        .await
        .ok()
        .unwrap();
    assert_eq!(
        server.receive::<String>(deadline()).await.unwrap(),
        Incoming::Message(SAMPLE_PAYLOAD.into())
    );
}

#[tokio::test]
async fn oversized_encoding_is_never_misreported_as_an_executed_mutation() {
    let (left, _right) = duplex(PIPE_CAPACITY);
    let mut channel = Framed::new(left);
    let huge = SAMPLE_PAYLOAD.repeat(MAX_FRAME_BYTES / SAMPLE_PAYLOAD.len() + ONE_BYTE);
    let failure = send(&mut channel, &huge, true, deadline())
        .await
        .err()
        .unwrap();
    assert!(matches!(
        failure.error,
        ClientError::Transport(TransportError::FrameTooLarge)
    ));
    assert!(failure.reusable && channel.is_usable());
}

#[tokio::test]
async fn deadline_before_write_is_not_unknown_and_remains_reusable() {
    let (left, _right) = duplex(PIPE_CAPACITY);
    let mut channel = Framed::new(left);
    let failure = send(&mut channel, &SAMPLE_PAYLOAD, true, Instant::now())
        .await
        .err()
        .unwrap();
    assert!(matches!(
        failure.error,
        ClientError::Transport(TransportError::Deadline)
    ));
    assert!(failure.reusable && channel.is_usable());
    let (mut client, mut server) = connected().await;
    assert!(matches!(
        client.call(hub(), input(), Instant::now()).await,
        Err(ClientError::Transport(TransportError::Deadline))
    ));
    assert!(client.is_usable());
    let peer = async {
        let request = receive(&mut server).await;
        assert_eq!(request.fields().request_id, id(REQUEST_ID));
        respond(&mut server, &request, Reply::Input { duplicate: false }).await;
    };
    let (reply, ()) = tokio::join!(client.call(hub(), input(), deadline()), peer);
    assert!(matches!(reply, Ok(Reply::Input { duplicate: false })));
}
