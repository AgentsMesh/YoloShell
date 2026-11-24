use crate::support::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, HubPin, accept, connect};
use yolo_hub_identity::HubId;

fn owned(stream: tokio::io::DuplexStream, closed: Arc<AtomicUsize>) -> ClientIo {
    ClientIo::new(stream, move || {
        closed.fetch_add(CLOSED_ONCE, Ordering::SeqCst);
    })
}

#[tokio::test]
async fn owned_generic_io_releases_both_endpoints_exactly_once_after_multiplexed_use() {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let registry = AccessRegistry::new(configuration(pin.hub_id, client_public)).unwrap();
    let (left, right) = tokio::io::duplex(BUFFER_BYTES);
    let left_closed = Arc::new(AtomicUsize::default());
    let right_closed = Arc::new(AtomicUsize::default());
    let (client, server) = tokio::join!(
        connect(
            owned(left, left_closed.clone()),
            client_key,
            pin,
            deadline()
        ),
        accept(
            owned(right, right_closed.clone()),
            hub_key,
            registry.clone(),
            deadline()
        ),
    );
    let client = client.unwrap();
    let (server, mut incoming) = server.unwrap();
    let mut outgoing = client.open_channel(deadline()).await.unwrap();
    let mut admitted = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    outgoing.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    admitted.stream.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    assert_eq!(left_closed.load(Ordering::SeqCst), usize::default());
    client.close(deadline()).await.unwrap();
    server.close(deadline()).await.unwrap();
    drop((client, server, outgoing, admitted));
    assert_eq!(left_closed.load(Ordering::SeqCst), CLOSED_ONCE);
    assert_eq!(right_closed.load(Ordering::SeqCst), CLOSED_ONCE);
}

#[tokio::test]
async fn cancelling_either_handshake_closes_owned_io_even_if_russh_task_is_pending() {
    for accepting in [false, true] {
        let (public, key) = keys();
        let pin = HubPin {
            hub_id: HubId::new(),
            public_key: public,
        };
        let registry = AccessRegistry::new(configuration(pin.hub_id, public)).unwrap();
        let (stream, mut remote) = tokio::io::duplex(BUFFER_BYTES);
        let closed = Arc::new(AtomicUsize::default());
        let transport = owned(stream, closed.clone());
        let opening = tokio::spawn(async move {
            if accepting {
                accept(transport, key, registry, deadline())
                    .await
                    .map(|_| ())
            } else {
                connect(transport, key, pin, deadline()).await.map(|_| ())
            }
        });
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .unwrap();
        opening.abort();
        assert!(opening.await.unwrap_err().is_cancelled());
        assert_eq!(closed.load(Ordering::SeqCst), CLOSED_ONCE);
        let mut remaining = Vec::new();
        timeout_at(deadline(), remote.read_to_end(&mut remaining))
            .await
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn wrong_hub_pin_retires_both_generic_io_owners() {
    let (client_public, client_key) = keys();
    let (_, hub_key) = keys();
    let (wrong_public, _) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: wrong_public,
    };
    let registry = AccessRegistry::new(configuration(pin.hub_id, client_public)).unwrap();
    let (left, right) = tokio::io::duplex(BUFFER_BYTES);
    let left_closed = Arc::new(AtomicUsize::default());
    let right_closed = Arc::new(AtomicUsize::default());
    let (client, server) = tokio::join!(
        connect(
            owned(left, left_closed.clone()),
            client_key,
            pin,
            deadline()
        ),
        accept(
            owned(right, right_closed.clone()),
            hub_key,
            registry.clone(),
            deadline()
        ),
    );
    assert!(client.is_err());
    assert!(server.is_err());
    assert_eq!(left_closed.load(Ordering::SeqCst), CLOSED_ONCE);
    assert_eq!(right_closed.load(Ordering::SeqCst), CLOSED_ONCE);
}
