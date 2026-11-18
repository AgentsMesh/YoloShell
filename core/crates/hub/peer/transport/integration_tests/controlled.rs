use crate::support::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_peer_transport::{PeerIo, PeerPins, accept_io, connect_io};

const BUFFER_BYTES: usize = 65_536;
const CLOSED_ONCE: usize = 1;

fn owned(stream: tokio::io::DuplexStream, closed: Arc<AtomicUsize>) -> PeerIo {
    PeerIo::new(stream, move || {
        closed.fetch_add(CLOSED_ONCE, Ordering::SeqCst);
    })
}

#[tokio::test]
async fn generic_io_keeps_inner_auth_and_independent_channels_until_explicit_close() {
    let (left_id, left_key) = keys();
    let (right_id, right_key) = keys();
    let (left, right) = tokio::io::duplex(BUFFER_BYTES);
    let left_closed = Arc::new(AtomicUsize::default());
    let right_closed = Arc::new(AtomicUsize::default());
    let (left, right) = tokio::join!(
        connect_io(
            owned(left, left_closed.clone()),
            left_key,
            right_id,
            deadline()
        ),
        accept_io(
            owned(right, right_closed.clone()),
            right_key,
            PeerPins::new(vec![left_id]).unwrap(),
            deadline()
        ),
    );
    let (left, _) = left.unwrap();
    let (right, mut incoming) = right.unwrap();
    let discarded = left.open_channel(deadline()).await.unwrap();
    let discarded_remote = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    drop((discarded, discarded_remote));
    let mut second = left.open_channel(deadline()).await.unwrap();
    let mut remote = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    second.write_all(PAYLOAD).await.unwrap();
    let mut received = vec![u8::default(); PAYLOAD.len()];
    timeout_at(deadline(), remote.read_exact(&mut received))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received, PAYLOAD);
    assert_eq!(left_closed.load(Ordering::SeqCst), usize::default());
    left.close(deadline()).await.unwrap();
    assert!(
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    right.close(deadline()).await.unwrap();
    drop((left, right, second));
    assert_eq!(left_closed.load(Ordering::SeqCst), CLOSED_ONCE);
    assert_eq!(right_closed.load(Ordering::SeqCst), CLOSED_ONCE);
}

#[tokio::test]
async fn generic_io_rejects_wrong_inner_hub_pin_and_releases_both_owners() {
    let (left_id, left_key) = keys();
    let (_, right_key) = keys();
    let (wrong_pin, _) = keys();
    let (left, right) = tokio::io::duplex(BUFFER_BYTES);
    let left_closed = Arc::new(AtomicUsize::default());
    let right_closed = Arc::new(AtomicUsize::default());
    let (left, right) = tokio::join!(
        connect_io(
            owned(left, left_closed.clone()),
            left_key,
            wrong_pin,
            deadline()
        ),
        accept_io(
            owned(right, right_closed.clone()),
            right_key,
            PeerPins::new(vec![left_id]).unwrap(),
            deadline()
        ),
    );
    assert!(left.is_err());
    assert!(right.is_err());
    assert_eq!(left_closed.load(Ordering::SeqCst), CLOSED_ONCE);
    assert_eq!(right_closed.load(Ordering::SeqCst), CLOSED_ONCE);
}

#[tokio::test]
async fn cancelling_an_incomplete_handshake_closes_its_transport_once() {
    for accepting in [false, true] {
        let (peer, key) = keys();
        let (stream, mut remote) = tokio::io::duplex(BUFFER_BYTES);
        let closed = Arc::new(AtomicUsize::default());
        let io = owned(stream, closed.clone());
        let opening = tokio::spawn(async move {
            if accepting {
                accept_io(io, key, PeerPins::new(vec![peer]).unwrap(), deadline()).await
            } else {
                connect_io(io, key, peer, deadline()).await
            }
        });
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .unwrap();
        opening.abort();
        assert!(matches!(opening.await, Err(error) if error.is_cancelled()));
        assert_eq!(closed.load(Ordering::SeqCst), CLOSED_ONCE);
        let mut remaining = Vec::new();
        timeout_at(deadline(), remote.read_to_end(&mut remaining))
            .await
            .unwrap()
            .unwrap();
    }
}
