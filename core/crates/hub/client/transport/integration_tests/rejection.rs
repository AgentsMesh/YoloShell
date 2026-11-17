use crate::support::*;
use russh::keys::PrivateKeyWithHashAlg;
use std::sync::Arc;
use tokio::time::timeout_at;
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, HubPin, PUBLIC_CLIENT_USER, accept, connect};
use yolo_hub_identity::HubId;

#[tokio::test]
async fn both_enrolled_hub_key_and_signed_enrolled_client_key_are_required() {
    for wrong_server in [true, false] {
        let (client_public, client_key) = keys();
        let (hub_public, hub_key) = keys();
        let (wrong_public, _) = keys();
        let hub_id = HubId::new();
        let pin = HubPin {
            hub_id,
            public_key: if wrong_server {
                wrong_public
            } else {
                hub_public
            },
        };
        let registry = AccessRegistry::new(configuration(
            hub_id,
            if wrong_server {
                client_public
            } else {
                wrong_public
            },
        ))
        .unwrap();
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
        assert!(client.is_err());
        assert!(server.is_err());
    }
}

#[tokio::test]
async fn private_federation_username_cannot_authenticate_with_an_enrolled_public_key() {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let registry = AccessRegistry::new(configuration(pin.hub_id, client_public)).unwrap();
    let (left, right) = sockets().await;
    let server = tokio::spawn(accept(
        ClientIo::from_tcp(right).unwrap(),
        hub_key,
        registry.clone(),
        deadline(),
    ));
    let mut client = russh::client::connect_stream(
        Arc::new(russh::client::Config::default()),
        left,
        raw::ClientHandler(pin),
    )
    .await
    .unwrap();
    let private_auth = timeout_at(
        deadline(),
        client.authenticate_publickey(
            PRIVATE_PEER_USER,
            PrivateKeyWithHashAlg::new(client_key.clone(), None),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!private_auth.success());
    assert!(!server.is_finished());
    let public_auth = timeout_at(
        deadline(),
        client.authenticate_publickey(
            PUBLIC_CLIENT_USER,
            PrivateKeyWithHashAlg::new(client_key, None),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(public_auth.success());
    let (server, _) = timeout_at(deadline(), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    server.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn authenticated_public_client_cannot_use_inner_transport_as_tcp_or_unix_proxy() {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let registry = AccessRegistry::new(configuration(pin.hub_id, client_public)).unwrap();
    let (left, right) = sockets().await;
    let server = tokio::spawn(accept(
        ClientIo::from_tcp(right).unwrap(),
        hub_key,
        registry.clone(),
        deadline(),
    ));
    let mut client = russh::client::connect_stream(
        Arc::new(russh::client::Config::default()),
        left,
        raw::ClientHandler(pin),
    )
    .await
    .unwrap();
    assert!(
        client
            .authenticate_publickey(
                PUBLIC_CLIENT_USER,
                PrivateKeyWithHashAlg::new(client_key, None)
            )
            .await
            .unwrap()
            .success()
    );
    let (server, _) = timeout_at(deadline(), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        timeout_at(
            deadline(),
            client.channel_open_direct_streamlocal(DENIED_SOCKET)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            client.channel_open_direct_tcpip(DENIED_HOST, DENIED_PORT, DENIED_HOST, DENIED_PORT)
        )
        .await
        .unwrap()
        .is_err()
    );
    server.close(deadline()).await.unwrap();
}
