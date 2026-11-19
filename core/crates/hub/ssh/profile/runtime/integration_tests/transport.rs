use super::{broker, constants::*, fixture::*};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_federation_settings::{PeerAddress, SshProfileRef};
use yolo_hub_peer_dialer::dial_peer;
use yolo_hub_peer_transport::{PeerIo, PeerPins, accept_io, connect_io};

#[tokio::test]
async fn unknown_profile_never_falls_back_to_the_loopback_tcp_endpoint() {
    let fixture = Fixture::new();
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = PeerAddress::ssh(
        SshProfileRef::new(PROFILE_ID, DIGEST_CHARACTER.repeat(DIGEST_BYTES)).unwrap(),
        listener.local_addr().unwrap(),
    )
    .unwrap();
    assert!(
        dial_peer(
            fixture.runtime.as_ref(),
            fixture.state.peer_identity(),
            &address,
            deadline()
        )
        .await
        .is_err()
    );
    let incoming =
        tokio::select! { _ = listener.accept() => true, _ = tokio::task::yield_now() => false };
    assert!(!incoming);
}

#[tokio::test]
async fn registry_removal_closes_the_existing_tunnel_before_the_change_reply() {
    let fixture = Fixture::new();
    let (paths, listener) = broker::bind(&fixture.root.path().join(BROKER_DIRECTORY));
    let profile = fixture.profile(paths.state_dir());
    fixture.apply(vec![profile.clone()]).await;
    let origin = yolo_hub_state::HubState::open(
        &fixture.root.path().join(ORIGIN_DIRECTORY),
        &fixture.root.path().join(ORIGIN_HOST_DIRECTORY),
    )
    .unwrap();
    let peer = origin.peer_identity();
    let address = PeerAddress::ssh(profile.profile, ENDPOINT.parse().unwrap()).unwrap();
    let (transport, remote) = tokio::join!(
        dial_peer(fixture.runtime.as_ref(), peer, &address, deadline()),
        timeout_at(deadline(), broker::upgrade(&listener, peer)),
    );
    let (transport, remote) = (transport.unwrap(), remote.unwrap());
    let (client, server) = tokio::join!(
        connect_io(
            transport,
            Arc::new(fixture.state.transport_key().unwrap()),
            peer,
            deadline()
        ),
        accept_io(
            PeerIo::new(remote, || {}),
            Arc::new(origin.transport_key().unwrap()),
            PeerPins::new(vec![fixture.state.peer_identity()]).unwrap(),
            deadline()
        ),
    );
    let (client, _) = client.unwrap();
    let (server, mut incoming) = server.unwrap();
    let mut transport = client.open_channel(deadline()).await.unwrap();
    let mut remote = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    transport.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![0; PAYLOAD.len()];
    timeout_at(deadline(), remote.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    fixture.apply(Vec::new()).await;
    let mut one = [0];
    assert_eq!(
        timeout_at(deadline(), remote.read(&mut one))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    timeout_at(deadline(), async {
        while !client.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(client.open_channel(deadline()).await.is_err());
    let _ = client.close(deadline()).await;
    let _ = server.close(deadline()).await;
    paths.verify_state().unwrap();
}
