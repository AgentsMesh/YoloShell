use crate::support::*;
use tokio::time::timeout_at;
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, HubPin, MAX_CLIENT_CHANNELS, accept, connect};
use yolo_hub_identity::HubId;

#[tokio::test]
async fn retained_channel_capacity_is_bounded_and_released_with_each_channel_owner() {
    let (client, server, mut incoming, _registry) = pair().await;
    let mut retained = Vec::new();
    for _ in 0..MAX_CLIENT_CHANNELS {
        let opened = client.open_channel(deadline()).await.unwrap();
        let admitted = timeout_at(deadline(), incoming.recv())
            .await
            .unwrap()
            .unwrap();
        retained.push((opened, admitted));
    }
    assert!(client.open_channel(deadline()).await.is_err());
    assert!(!client.is_closed());
    drop(retained.pop().unwrap());
    let recovered = client.open_channel(deadline()).await.unwrap();
    let admitted = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    drop((recovered, admitted, retained));
    client.close(deadline()).await.unwrap();
    server.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn maximum_epoch_expiry_does_not_overflow_tokio_timer_or_skip_revocation() {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let mut config = configuration(pin.hub_id, client_public);
    config.clients.first_mut().unwrap().expires_at_ms = u64::MAX;
    let registry = AccessRegistry::new(config).unwrap();
    let (left, right) = sockets().await;
    let (client, server) = tokio::join!(
        connect(
            ClientIo::from_tcp(left).unwrap(),
            client_key,
            pin,
            deadline()
        ),
        accept(
            ClientIo::from_tcp(right).unwrap(),
            hub_key,
            registry.clone(),
            deadline()
        ),
    );
    let client = client.unwrap();
    let (server, mut incoming) = server.unwrap();
    let channel = client.open_channel(deadline()).await.unwrap();
    let admitted = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(admitted.admission.is_current(now_ms()));
    registry.close();
    timeout_at(deadline(), server.closed()).await.unwrap();
    timeout_at(deadline(), client.closed()).await.unwrap();
    drop((channel, admitted));
    client.close(deadline()).await.unwrap();
    server.close(deadline()).await.unwrap();
}
