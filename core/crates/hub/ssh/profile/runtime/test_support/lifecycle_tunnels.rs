use super::super::{
    broker,
    constants::*,
    fixture::{Fixture, deadline},
    lifecycle_constants::*,
};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
    time::{timeout, timeout_at},
};
use yolo_hub_federation_settings::{PeerAddress, SshProfileRef};
use yolo_hub_peer_dialer::dial_peer;
use yolo_hub_peer_transport::{
    PeerConnection, PeerIo, PeerPins, PeerStream, accept_io, connect_io,
};
use yolo_hub_state::HubState;
use yolo_remote_broker::BrokerPaths;

pub struct Tunnels {
    connections: Vec<(PeerConnection, PeerConnection)>,
    _streams: Vec<(PeerStream, PeerStream)>,
    _brokers: Vec<(BrokerPaths, UnixListener)>,
    _peer: HubState,
}
impl Tunnels {
    pub async fn open(fixture: &Fixture) -> Self {
        let peer = HubState::open(
            &fixture.root.path().join(PEER_STATE_DIRECTORY),
            &fixture.root.path().join(PEER_HOST_DIRECTORY),
        )
        .unwrap();
        let brokers = (0..TUNNEL_COUNT)
            .map(|index| {
                broker::bind(
                    &fixture
                        .root
                        .path()
                        .join(format!("{BROKER_DIRECTORY}-{index}")),
                )
            })
            .collect::<Vec<_>>();
        let profiles = brokers
            .iter()
            .enumerate()
            .map(|(index, (paths, _))| {
                let mut profile = fixture.profile(paths.state_dir());
                profile.profile = SshProfileRef::new(
                    format!("{PROFILE_ID}-{index}"),
                    DIGEST_CHARACTER.repeat(DIGEST_BYTES),
                )
                .unwrap();
                profile
            })
            .collect::<Vec<_>>();
        fixture.apply(profiles.clone()).await;
        let mut connections = Vec::new();
        let mut streams = Vec::new();
        for (profile, (_, listener)) in profiles.into_iter().zip(&brokers) {
            let address = PeerAddress::ssh(profile.profile, ENDPOINT.parse().unwrap()).unwrap();
            let remote = peer.peer_identity();
            let (opened, accepted) = tokio::join!(
                dial_peer(fixture.runtime.as_ref(), remote, &address, deadline()),
                timeout_at(deadline(), broker::upgrade(listener, remote)),
            );
            let (client, server) = tokio::join!(
                connect_io(
                    opened.unwrap(),
                    Arc::new(fixture.state.transport_key().unwrap()),
                    remote,
                    deadline()
                ),
                accept_io(
                    PeerIo::new(accepted.unwrap(), || {}),
                    Arc::new(peer.transport_key().unwrap()),
                    PeerPins::new(vec![fixture.state.peer_identity()]).unwrap(),
                    deadline()
                ),
            );
            let (client, _) = client.unwrap();
            let (server, mut incoming) = server.unwrap();
            assert_eq!(client.peer(), remote);
            assert_eq!(server.peer(), fixture.state.peer_identity());
            let mut stream = client.open_channel(deadline()).await.unwrap();
            let mut accepted = timeout_at(deadline(), incoming.recv())
                .await
                .unwrap()
                .unwrap();
            stream.write_all(PAYLOAD).await.unwrap();
            let mut bytes = vec![0; PAYLOAD.len()];
            timeout_at(deadline(), accepted.read_exact(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(bytes, PAYLOAD);
            assert!(!client.is_closed() && !server.is_closed());
            connections.push((client, server));
            streams.push((stream, accepted));
        }
        Self {
            connections,
            _streams: streams,
            _brokers: brokers,
            _peer: peer,
        }
    }
    /// Socket shutdown is synchronous; russh publishes handler closure asynchronously.
    pub async fn require_closed(&self) {
        timeout(CLOSURE_OBSERVATION, async {
            while self
                .connections
                .iter()
                .any(|(local, remote)| !local.is_closed() || !remote.is_closed())
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        for (local, remote) in &self.connections {
            assert!(local.open_channel(deadline()).await.is_err());
            assert!(remote.open_channel(deadline()).await.is_err());
        }
    }
    pub async fn finish(&self) {
        for (local, remote) in &self.connections {
            let (left, right) = tokio::join!(local.close(deadline()), remote.close(deadline()));
            left.unwrap();
            right.unwrap();
        }
    }
}
