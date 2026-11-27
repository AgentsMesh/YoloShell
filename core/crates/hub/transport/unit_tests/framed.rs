use crate::support::*;
use tokio::io::{AsyncWriteExt, duplex};
use tokio::time::Instant;
use yolo_hub_transport::*;

#[tokio::test]
async fn orderly_eof_closes_without_being_mistaken_for_a_truncated_message() {
    let (left, right) = duplex(PIPE_CAPACITY);
    drop(left);
    let mut client = Framed::new(right);
    assert_eq!(
        client.receive::<Message>(deadline()).await.unwrap(),
        Incoming::Eof
    );
    assert!(!client.is_usable());
    assert!(matches!(
        client.receive::<Message>(deadline()).await,
        Err(TransportError::Poisoned)
    ));
    assert!(matches!(
        client.send(&message(), deadline()).await,
        Err(TransportError::Poisoned)
    ));
    assert!(matches!(
        client.close(deadline()).await,
        Err(TransportError::Poisoned)
    ));
}

#[tokio::test]
async fn truncated_headers_and_payloads_cannot_be_reused() {
    let encoded = encode(&message()).unwrap();
    for length in 1..encoded.len() {
        let (mut server, right) = duplex(encoded.len());
        server.write_all(&encoded[..length]).await.unwrap();
        drop(server);
        let mut client = Framed::new(right);
        assert!(
            matches!(
                client.receive::<Message>(deadline()).await,
                Err(TransportError::TruncatedFrame)
            ),
            "{length}"
        );
        assert!(!client.is_usable());
    }
}

#[tokio::test]
async fn zero_and_oversized_length_are_rejected_before_waiting_for_payload() {
    for length in [0, MAX_FRAME_BYTES + 1] {
        let (mut server, right) = duplex(PIPE_CAPACITY);
        server
            .write_all(&(length as u32).to_be_bytes())
            .await
            .unwrap();
        let mut client = Framed::new(right);
        let error = client.receive::<Message>(deadline()).await.unwrap_err();
        assert!(matches!(
            error,
            TransportError::EmptyFrame | TransportError::FrameTooLarge
        ));
        assert!(!client.is_usable());
    }
}

#[tokio::test(start_paused = true)]
async fn mid_transfer_deadlines_poison_but_preflight_deadlines_do_not() {
    let (left, _right) = duplex(PIPE_CAPACITY);
    let mut client = Framed::new(left);
    assert!(matches!(
        client.receive::<Message>(Instant::now()).await,
        Err(TransportError::Deadline)
    ));
    assert!(matches!(
        client.send(&message(), Instant::now()).await,
        Err(TransportError::Deadline)
    ));
    assert!(client.is_usable());
    assert!(matches!(
        client.receive::<Message>(deadline()).await,
        Err(TransportError::Deadline)
    ));
    assert!(!client.is_usable());
    let (left, _right) = duplex(1);
    let mut client = Framed::new(left);
    assert!(matches!(
        client.send(&message(), deadline()).await,
        Err(TransportError::Deadline)
    ));
    assert!(!client.is_usable());
}

#[tokio::test]
async fn write_failures_and_clean_shutdown_retire_connections() {
    let (left, right) = duplex(PIPE_CAPACITY);
    drop(right);
    let mut client = Framed::new(left);
    assert!(matches!(
        client.send(&message(), deadline()).await,
        Err(TransportError::Io(_))
    ));
    assert!(!client.is_usable());
    let (left, mut right) = duplex(PIPE_CAPACITY);
    let mut client = Framed::new(left);
    client.close(deadline()).await.unwrap();
    assert!(!client.is_usable());
    let mut byte = [0];
    assert_eq!(
        tokio::io::AsyncReadExt::read(&mut right, &mut byte)
            .await
            .unwrap(),
        0
    );
}
