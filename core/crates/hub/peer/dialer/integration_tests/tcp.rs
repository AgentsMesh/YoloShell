use super::{constants::*, fixture::*, support::*};
use std::{net::Ipv4Addr, num::NonZeroU16};
use tokio::{
    io::AsyncReadExt,
    net::TcpListener,
    time::{Instant, timeout, timeout_at},
};
use yolo_hub_federation_settings::{PeerAddress, SshProfileRef};
use yolo_hub_peer_dialer::{PeerDialer, TcpPeerDialer, dial_peer};

#[tokio::test]
async fn default_tcp_returns_owned_transport_whose_drop_closes_the_actual_socket() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, DYNAMIC_PORT))
        .await
        .unwrap();
    let endpoint = listener.local_addr().unwrap();
    let address = PeerAddress::tcp(
        endpoint.ip().to_string(),
        NonZeroU16::new(endpoint.port()).unwrap(),
    );
    let peer = keys().0;
    let until = deadline();
    let (transport, accepted) = tokio::join!(
        dial_peer(&TcpPeerDialer, peer, &address, until),
        timeout_at(until, listener.accept())
    );
    let transport = transport.unwrap();
    let (mut remote, _) = accepted.unwrap().unwrap();
    drop(transport);
    assert!(
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
}

#[tokio::test]
async fn unsupported_ssh_or_failed_injected_binding_never_falls_back_to_tcp() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, DYNAMIC_PORT))
        .await
        .unwrap();
    let endpoint = listener.local_addr().unwrap();
    let peer = keys().0;
    let address = PeerAddress::ssh(
        SshProfileRef::new(PROFILE, PROFILE_REVISION).unwrap(),
        endpoint,
    )
    .unwrap();
    assert!(
        dial_peer(&TcpPeerDialer, peer, &address, deadline())
            .await
            .is_err()
    );
    let unavailable = Injected::new(None, Behavior::Reject);
    assert!(
        dial_peer(&unavailable, peer, &address, deadline())
            .await
            .is_err()
    );
    assert!(timeout(SHORT_TIMEOUT, listener.accept()).await.is_err());
    assert!(
        TcpPeerDialer
            .dial(peer, &super::fixture::address(), Instant::now())
            .await
            .is_err()
    );
}
