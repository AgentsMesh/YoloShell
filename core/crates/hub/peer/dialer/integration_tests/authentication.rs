use super::{constants::*, fixture::*, support::*};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_peer_dialer::dial_peer;
use yolo_hub_peer_transport::{PeerIo, PeerPins, accept_io, connect_io};

#[tokio::test]
async fn injected_io_still_requires_both_pins_and_preserves_independent_peer_channels() {
    let (local, local_key) = keys();
    let (peer, peer_key) = keys();
    let (left, right) = tokio::io::duplex(BUFFER_BYTES);
    let closed = Arc::new(AtomicUsize::default());
    let observed = closed.clone();
    let factory = Injected::new(
        Some(PeerIo::new(left, move || {
            observed.fetch_add(ONE, Ordering::SeqCst);
        })),
        Behavior::Ready,
    );
    let address = address();
    let deadline = deadline();
    let (connected, accepted) = tokio::join!(
        async {
            connect_io(
                dial_peer(&factory, peer, &address, deadline).await.unwrap(),
                local_key,
                peer,
                deadline,
            )
            .await
        },
        accept_io(
            PeerIo::new(right, || {}),
            peer_key,
            PeerPins::new(vec![local]).unwrap(),
            deadline
        ),
    );
    let (client, _) = connected.unwrap();
    let (server, mut incoming) = accepted.unwrap();
    assert_eq!(client.peer(), peer);
    assert_eq!(server.peer(), local);
    let mut stream = client.open_channel(deadline).await.unwrap();
    let mut remote = timeout_at(deadline, incoming.recv())
        .await
        .unwrap()
        .unwrap();
    stream.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![0; PAYLOAD.len()];
    timeout_at(deadline, remote.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    client.close(deadline).await.unwrap();
    server.close(deadline).await.unwrap();
    assert_eq!(closed.load(Ordering::SeqCst), ONE);
}

#[tokio::test]
async fn injected_factory_cannot_replace_the_configured_hub_public_key() {
    let (local, local_key) = keys();
    let (_, actual_key) = keys();
    let (configured, _) = keys();
    let (left, right) = tokio::io::duplex(BUFFER_BYTES);
    let closed = Arc::new(AtomicUsize::default());
    let observed = closed.clone();
    let factory = Injected::new(
        Some(PeerIo::new(left, move || {
            observed.fetch_add(ONE, Ordering::SeqCst);
        })),
        Behavior::Ready,
    );
    let address = address();
    let deadline = deadline();
    let (connected, accepted) = tokio::join!(
        async {
            connect_io(
                dial_peer(&factory, configured, &address, deadline)
                    .await
                    .unwrap(),
                local_key,
                configured,
                deadline,
            )
            .await
        },
        accept_io(
            PeerIo::new(right, || {}),
            actual_key,
            PeerPins::new(vec![local]).unwrap(),
            deadline
        ),
    );
    assert!(connected.is_err());
    assert!(accepted.is_err());
    assert_eq!(closed.load(Ordering::SeqCst), ONE);
}
