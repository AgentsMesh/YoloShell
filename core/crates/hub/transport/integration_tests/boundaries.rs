use crate::support::*;
use tokio::io::{AsyncWriteExt, duplex};
use yolo_hub_transport::*;

#[tokio::test]
async fn backpressured_bidirectional_stream_transfers_complete_frames_in_order() {
    let (left, right) = duplex(1);
    let mut client = Framed::new(left);
    let mut server = Framed::new(right);
    let payload = message();
    let (sent, received) = tokio::join!(
        client.send(&payload, deadline()),
        server.receive::<Message>(deadline())
    );
    sent.unwrap();
    assert_eq!(received.unwrap(), Incoming::Message(message()));
    assert!(client.is_usable() && server.is_usable());
    let (sent, received) = tokio::join!(
        server.send(&payload, deadline()),
        client.receive::<Message>(deadline())
    );
    sent.unwrap();
    assert_eq!(received.unwrap(), Incoming::Message(message()));
}

#[tokio::test]
async fn malformed_frame_cannot_be_skipped_to_consume_a_valid_successor() {
    let malformed = encode(&false).unwrap();
    let successor = encode(&message()).unwrap();
    let (mut left, right) = duplex(malformed.len() + successor.len());
    left.write_all(&malformed).await.unwrap();
    left.write_all(&successor).await.unwrap();
    let mut connection = Framed::new(right);
    assert!(matches!(
        connection.receive::<Message>(deadline()).await,
        Err(TransportError::Decode(_))
    ));
    assert!(matches!(
        connection.receive::<Message>(deadline()).await,
        Err(TransportError::Poisoned)
    ));
}

#[tokio::test(start_paused = true)]
async fn cancellation_during_partial_receive_closes_instead_of_reusing_the_stream() {
    let (mut left, right) = duplex(PIPE_CAPACITY);
    left.write_all(&[0]).await.unwrap();
    let mut client = Framed::new(right);
    tokio::select! {
        biased;
        _ = client.receive::<Message>(deadline()) => panic!("partial frame unexpectedly completed"),
        _ = tokio::task::yield_now() => {}
    }
    assert!(!client.is_usable());
    assert!(matches!(
        client.receive::<Message>(deadline()).await,
        Err(TransportError::Poisoned)
    ));
    assert!(left.write_all(&[0]).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn cancellation_during_partial_send_cannot_duplicate_the_remaining_input() {
    let (left, _right) = duplex(1);
    let mut client = Framed::new(left);
    let payload = message();
    tokio::select! {
        biased;
        _ = client.send(&payload, deadline()) => panic!("backpressured frame unexpectedly completed"),
        _ = tokio::task::yield_now() => {}
    }
    assert!(!client.is_usable());
    assert!(matches!(
        client.send(&message(), deadline()).await,
        Err(TransportError::Poisoned)
    ));
}
