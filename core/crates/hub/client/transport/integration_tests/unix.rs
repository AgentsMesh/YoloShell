use crate::support::*;
use tokio::{io::AsyncReadExt, net::UnixStream, time::timeout_at};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, HubPin, accept, connect};
use yolo_hub_identity::HubId;

#[tokio::test]
async fn unix_socket_custody_closes_remote_channels_and_notifies_connection_waiters() {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let registry = AccessRegistry::new(configuration(pin.hub_id, client_public)).unwrap();
    let (left, right) = UnixStream::pair().unwrap();
    let (client, server) = tokio::join!(
        connect(
            ClientIo::from_unix(left).unwrap(),
            client_key,
            pin,
            deadline()
        ),
        accept(
            ClientIo::from_unix(right).unwrap(),
            hub_key,
            registry.clone(),
            deadline()
        ),
    );
    let client = client.unwrap();
    let (server, mut incoming) = server.unwrap();
    let mut channel = client.open_channel(deadline()).await.unwrap();
    let admitted = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    server.close(deadline()).await.unwrap();
    timeout_at(deadline(), client.closed()).await.unwrap();
    assert!(
        timeout_at(deadline(), channel.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert!(!admitted.connection_id.is_nil());
    timeout_at(deadline(), server.closed()).await.unwrap();
    client.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn cancelling_handshake_shuts_down_the_actual_unix_socket() {
    for accepting in [true, false] {
        let (public, key) = keys();
        let pin = HubPin {
            hub_id: HubId::new(),
            public_key: public,
        };
        let registry = AccessRegistry::new(configuration(pin.hub_id, public)).unwrap();
        let (left, mut right) = UnixStream::pair().unwrap();
        let owned = ClientIo::from_unix(left).unwrap();
        let opening = tokio::spawn(async move {
            if accepting {
                accept(owned, key, registry, deadline()).await.map(|_| ())
            } else {
                connect(owned, key, pin, deadline()).await.map(|_| ())
            }
        });
        timeout_at(deadline(), right.read_u8())
            .await
            .unwrap()
            .unwrap();
        opening.abort();
        assert!(opening.await.unwrap_err().is_cancelled());
        let mut remaining = Vec::new();
        timeout_at(deadline(), right.read_to_end(&mut remaining))
            .await
            .unwrap()
            .unwrap();
    }
}
