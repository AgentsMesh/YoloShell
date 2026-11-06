use crate::support::*;
use std::{net::Shutdown, sync::Arc};
use tokio::{
    net::TcpStream,
    sync::oneshot,
    time::{timeout, timeout_at},
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, HubPin, PUBLIC_CLIENT_USER, accept};
use yolo_hub_identity::HubId;

#[tokio::test]
async fn public_key_offer_and_unsigned_authentication_never_create_an_admission() {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let registry = AccessRegistry::new(configuration(pin.hub_id, client_public)).unwrap();
    let (left, right) = sockets().await;
    let socket = left.into_std().unwrap();
    let closer = socket.try_clone().unwrap();
    let mut server = tokio::spawn(accept(
        ClientIo::from_tcp(right).unwrap(),
        hub_key,
        registry.clone(),
        deadline(),
    ));
    let (offered, observed) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let authenticating = tokio::spawn(async move {
        let mut client = russh::client::connect_stream(
            Arc::new(russh::client::Config::default()),
            TcpStream::from_std(socket).unwrap(),
            raw::ClientHandler(pin),
        )
        .await
        .unwrap();
        let mut signer = signer::Unsigned {
            offered: Some(offered),
            release: Some(released),
        };
        client
            .authenticate_publickey_with(
                PUBLIC_CLIENT_USER,
                client_key.public_key().clone(),
                None,
                &mut signer,
            )
            .await
    });
    timeout_at(deadline(), observed).await.unwrap().unwrap();
    assert!(timeout(PROBE_OBSERVATION, &mut server).await.is_err());
    release.send(()).unwrap();
    let result = timeout_at(deadline(), authenticating)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err() || !result.unwrap().success());
    let _ = closer.shutdown(Shutdown::Both);
    assert!(
        timeout_at(deadline(), server)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
}
