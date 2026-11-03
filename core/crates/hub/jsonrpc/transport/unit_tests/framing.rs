use serde_json::{Value, json};
use tokio::{
    io::{AsyncWriteExt, duplex},
    time::{Duration, Instant},
};
use yolo_hub_jsonrpc_transport::*;

const PIPE_BYTES: usize = 8 * 1_024;
const TIMEOUT: Duration = Duration::from_secs(3);
fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}

#[tokio::test]
async fn fragmented_and_coalesced_frames_survive_the_stream_handoff() {
    let (left, mut right) = duplex(PIPE_BYTES);
    let values = [json!({"first":"a\nb"}), json!({"second":true})];
    let combined: Vec<_> = values
        .iter()
        .flat_map(|value| encode(value).unwrap())
        .collect();
    for fragment in combined.chunks(3) {
        right.write_all(fragment).await.unwrap();
    }
    let mut framed = Framed::new(left);
    assert_eq!(
        framed.receive::<Value>(deadline()).await.unwrap(),
        Incoming::Message(values[0].clone())
    );
    let mut transferred = Framed::new(framed.into_stream().unwrap());
    assert_eq!(
        transferred.receive::<Value>(deadline()).await.unwrap(),
        Incoming::Message(values[1].clone())
    );
    right.shutdown().await.unwrap();
    assert_eq!(
        transferred.receive::<Value>(deadline()).await.unwrap(),
        Incoming::Eof
    );
    assert!(!transferred.is_usable());
}

#[tokio::test]
async fn malformed_empty_and_partial_documents_retire_the_connection() {
    for bytes in [
        b"\n".as_slice(),
        b"{}",
        b"{oops}\n",
        b"{\"x\":1,\"x\":2}\n",
        b"{}{}\n",
    ] {
        let (left, mut right) = duplex(PIPE_BYTES);
        right.write_all(bytes).await.unwrap();
        right.shutdown().await.unwrap();
        let mut framed = Framed::new(left);
        assert!(framed.receive::<Value>(deadline()).await.is_err());
        assert!(!framed.is_usable());
        assert!(matches!(
            framed.into_stream(),
            Err(TransportError::Poisoned)
        ));
    }
}

#[tokio::test]
async fn oversized_unterminated_input_is_bounded_and_serialization_is_preflight() {
    let (left, mut right) = duplex(PIPE_BYTES);
    let write = async {
        let bytes = vec![b'x'; PIPE_BYTES];
        while right.write_all(&bytes).await.is_ok() {}
    };
    let read = async {
        let mut framed = Framed::new(left);
        assert!(matches!(
            framed.receive::<Value>(deadline()).await,
            Err(TransportError::FrameTooLarge)
        ));
        assert!(!framed.is_usable());
    };
    tokio::join!(write, read);
    let huge = "x".repeat(MAX_FRAME_BYTES);
    assert!(matches!(encode(&huge), Err(TransportError::FrameTooLarge)));
}

#[tokio::test]
async fn invalid_json_frame_can_be_answered_with_a_protocol_error() {
    let (left, mut right) = duplex(PIPE_BYTES);
    right.write_all(b"{invalid}\n").await.unwrap();
    let mut framed = Framed::new(left);
    assert_eq!(
        framed.receive_frame(deadline()).await.unwrap(),
        Incoming::Message(b"{invalid}".to_vec())
    );
    assert!(framed.is_usable());
    framed
        .send(
            &json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}),
            deadline(),
        )
        .await
        .unwrap();
    let mut other = Framed::new(right);
    assert!(matches!(
        other.receive::<Value>(deadline()).await.unwrap(),
        Incoming::Message(_)
    ));
}
