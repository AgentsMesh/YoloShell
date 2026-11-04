use crate::{SshProfileRuntime, constants::*, monitor};
use anyhow::{Context, Result, ensure};
use tokio::time::Instant;
use yolo_broker_hub_protocol::BrokerHubOpenRequest;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::{DialTransport, PeerAddress};
use yolo_hub_peer_dialer::{PeerDialFuture, PeerDialer, TcpPeerDialer};
use yolo_hub_peer_transport::PeerIo;
use yolo_remote_broker::{BrokerClient, BrokerPaths};

impl PeerDialer for SshProfileRuntime {
    fn dial<'a>(
        &'a self,
        peer: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async move {
            self.state.verify().inspect_err(|_| self.close_all())?;
            address.validate()?;
            let DialTransport::Ssh { profile } = &address.transport else {
                return TcpPeerDialer.dial(peer, address, deadline).await;
            };
            let changed = self.changed.subscribe();
            let registration = self.registration(profile)?;
            let binding = {
                let (reserved, reservation) = self.reserve_profile(profile)?;
                ensure!(reserved == registration, REGISTRY_CHANGED);
                self.ensure_broker(&reservation, None, deadline).await?
            };
            let paths = BrokerPaths::open_existing(&registration.broker_state)?;
            paths.verify_state()?;
            let client = BrokerClient::new(paths.management_socket());
            let request = BrokerHubOpenRequest {
                binding,
                endpoint: address.ssh_endpoint()?,
                peer,
            };
            let tunnel = client.open_hub_peer(request, deadline).await?;
            paths.verify_state()?;
            self.state.verify().inspect_err(|_| self.close_all())?;
            ensure!(Instant::now() < deadline, REGISTRY_CHANGED);
            let close = tunnel.close_handle();
            let retained = {
                let store = self
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!(POISONED))
                    .inspect_err(|_| self.close_all())?;
                let state = store.current().inspect_err(|_| self.close_all())?;
                ensure!(
                    state.configuration.profiles.contains(&registration),
                    REGISTRY_CHANGED
                );
                self.retain_tunnel(registration.clone(), close.clone())?
            };
            let guard = monitor::watch(
                std::sync::Arc::downgrade(&self.state),
                std::sync::Arc::downgrade(&self.store),
                std::sync::Arc::downgrade(&self.active),
                registration,
                changed,
                close.clone(),
            );
            Ok(PeerIo::new(tunnel, move || {
                close.close();
                guard.abort();
                drop(retained);
            }))
        })
    }
}

impl SshProfileRuntime {
    fn registration(
        &self,
        profile: &yolo_hub_federation_settings::SshProfileRef,
    ) -> Result<yolo_hub_ssh_profiles::SshBrokerProfile> {
        let store = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!(POISONED))
            .inspect_err(|_| self.close_all())?;
        let state = store.current().inspect_err(|_| self.close_all())?;
        state
            .configuration
            .profiles
            .into_iter()
            .find(|entry| &entry.profile == profile)
            .context(REGISTRY_CHANGED)
    }
}
