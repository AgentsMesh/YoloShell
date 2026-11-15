use super::{constants::*, fixture::LinkFixture};
use tokio::{
    io::DuplexStream,
    time::{Instant, timeout_at},
};
use yolo_hub_peer_rpc::{Client, RpcError};
use yolo_hub_peer_transport::{ChannelAdmissionError, MAX_PEER_CHANNELS, PeerStream};

pub enum Pressure {
    Ssh(Vec<(PeerStream, PeerStream)>),
    Rpc(Vec<(Client, DuplexStream)>),
}

impl Pressure {
    pub async fn ssh(fixture: &mut LinkFixture) -> Self {
        let mut channels = Vec::new();
        loop {
            assert!(channels.len() <= MAX_PEER_CHANNELS);
            match fixture
                .link
                .connection
                .open_channel(Instant::now() + TEST_TIMEOUT)
                .await
            {
                Ok(outgoing) => {
                    let incoming = timeout_at(
                        Instant::now() + TEST_TIMEOUT,
                        fixture.peer_incoming.as_mut().unwrap().recv(),
                    )
                    .await
                    .unwrap()
                    .unwrap();
                    channels.push((outgoing, incoming));
                }
                Err(error) => {
                    assert_eq!(
                        error.downcast_ref::<ChannelAdmissionError>(),
                        Some(&ChannelAdmissionError::AtCapacity),
                    );
                    assert!(!channels.is_empty());
                    return Self::Ssh(channels);
                }
            }
        }
    }

    pub async fn rpc() -> Self {
        let mut connections = Vec::new();
        loop {
            assert!(connections.len() <= yolo_capnp_rpc::MAX_CONNECTIONS);
            let (local, remote) = tokio::io::duplex(DUPLEX_BYTES);
            match Client::connect(local, Instant::now() + TEST_TIMEOUT).await {
                Ok(client) => connections.push((client, remote)),
                Err(error) => {
                    assert_eq!(error, RpcError::Capacity);
                    assert!(!connections.is_empty());
                    return Self::Rpc(connections);
                }
            }
        }
    }

    pub fn retained(&self) -> usize {
        match self {
            Self::Ssh(channels) => channels.len(),
            Self::Rpc(connections) => connections.len(),
        }
    }
}
