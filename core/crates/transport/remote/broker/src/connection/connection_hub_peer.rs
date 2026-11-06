use super::{BrokerConnectionPool, connection::ObservedConnection};
use crate::remote::{ChannelPurpose, HubPeerEndpoint, HubPeerRequest, OpenedHubPeerChannel};
use anyhow::{Result, ensure};
use yolo_broker_hub_protocol::{
    BrokerHubConnectionBinding, BrokerHubError, BrokerHubOpenRequest, BrokerHubProfile,
};

impl BrokerConnectionPool {
    pub(super) async fn prepare_hub_transport(
        &self,
        expected: &BrokerHubProfile,
    ) -> Result<BrokerHubConnectionBinding> {
        self.require_hub_profile(expected)?;
        let observed = self.observed_connection().await?;
        ensure!(!self.is_retired(&observed), BrokerHubError::StaleBinding);
        Ok(self.hub_binding(expected.clone(), &observed))
    }

    pub(super) async fn open_hub_peer(
        &self,
        request: &BrokerHubOpenRequest,
    ) -> Result<OpenedHubPeerChannel> {
        request.validate()?;
        self.require_hub_profile(&request.binding.profile)?;
        let observed = self
            .cached_connection()
            .await
            .ok_or(BrokerHubError::StaleBinding)?;
        ensure!(
            self.hub_binding(request.binding.profile.clone(), &observed) == request.binding,
            BrokerHubError::StaleBinding
        );
        let remote = HubPeerRequest::new(
            observed.id(),
            HubPeerEndpoint::new(request.socket.clone())?,
            request.peer,
        )?;
        let opened = match observed.connection().open_hub_peer(&remote).await {
            Ok(opened) => opened,
            Err(error) => {
                if error.requires_connection_recovery() {
                    self.invalidate(&observed).await;
                }
                return Err(error.into());
            }
        };
        ensure!(
            opened.identity.connection_id == observed.id()
                && opened.identity.purpose == ChannelPurpose::HubPeer
                && opened.peer == request.peer,
            BrokerHubError::ChannelMismatch
        );
        ensure!(!self.is_retired(&observed), BrokerHubError::StaleBinding);
        Ok(opened)
    }

    pub(super) fn hub_binding(
        &self,
        profile: BrokerHubProfile,
        observed: &ObservedConnection,
    ) -> BrokerHubConnectionBinding {
        BrokerHubConnectionBinding {
            profile,
            broker_instance_id: self.instance_id(),
            connection_id: observed.id(),
            connection_generation: observed.generation(),
        }
    }
}
