use crate::support::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, HubPin, accept, connect};
use yolo_hub_identity::HubId;

#[tokio::test]
async fn registry_replacement_close_and_last_owner_drop_retire_already_open_channels() {
    for mode in [Revoke::Replace, Revoke::Close, Revoke::Drop] {
        let (client, server, mut incoming, registry) = pair().await;
        let mut outgoing = client.open_channel(deadline()).await.unwrap();
        let admitted = timeout_at(deadline(), incoming.recv())
            .await
            .unwrap()
            .unwrap();
        match mode {
            Revoke::Replace => {
                let old = registry.current().unwrap();
                let mut next = old.clone();
                next.revision += REVISION_STEP;
                registry.replace(old.revision, next).unwrap();
            }
            Revoke::Close => registry.close(),
            Revoke::Drop => drop(registry),
        }
        assert!(
            timeout_at(deadline(), outgoing.read_u8())
                .await
                .unwrap()
                .is_err()
        );
        assert!(!admitted.admission.is_current(now_ms()));
        assert!(client.open_channel(deadline()).await.is_err());
        assert!(server.is_closed());
        client.close(deadline()).await.unwrap();
        server.close(deadline()).await.unwrap();
    }
}

enum Revoke {
    Replace,
    Close,
    Drop,
}

#[tokio::test]
async fn expiry_closes_an_idle_admitted_stream_without_waiting_for_another_rpc() {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let mut config = configuration(pin.hub_id, client_public);
    config.clients.first_mut().unwrap().expires_at_ms =
        now_ms() + u64::try_from(SHORT_GRANT.as_millis()).unwrap();
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
    let mut stream = client.open_channel(deadline()).await.unwrap();
    let mut admitted = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    admitted.stream.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    stream.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    assert!(
        timeout_at(deadline(), stream.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert!(server.is_closed());
    client.close(deadline()).await.unwrap();
    server.close(deadline()).await.unwrap();
}
