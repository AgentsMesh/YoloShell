use crate::{configuration, constants::*};
use anyhow::{Result, bail, ensure};
use tokio::time::Instant;
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_client::{Client, ClientError, LocalClient};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_local_startup::ConnectionDocument;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_remote_provisioning_protocol::{BootstrapError, RemoteBootstrapRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

/// One authenticated socket for the whole observation/submission. No method reconnects.
pub(crate) struct LiveControl {
    client: LocalClient,
    peer: PeerIdentity,
}
impl LiveControl {
    pub async fn open(
        connection: &ConnectionDocument,
        peer: PeerIdentity,
        deadline: Instant,
    ) -> Result<Self> {
        ensure!(connection.entry_hub == peer.hub(), PIN_MISMATCH);
        let client = Client::connect_local(&connection.socket_path, peer.hub(), deadline).await?;
        Ok(Self { client, peer })
    }
    pub fn is_usable(&self) -> bool {
        self.client.is_usable()
    }
    pub async fn observe(
        &mut self,
        request: &RemoteBootstrapRequest,
        deadline: Instant,
    ) -> Result<(ServiceBuild, ConfigurationSnapshot)> {
        let build = self.build(deadline).await?;
        configuration::verify_images(&build, request)?;
        let snapshot = self.snapshot(deadline).await?;
        ensure!(snapshot.state.identity == self.peer, PIN_MISMATCH);
        let after = self.build(deadline).await?;
        configuration::verify_images(&after, request)?;
        ensure!(build == after, BootstrapError::UpgradeRequired);
        Ok((build, snapshot))
    }
    async fn build(&mut self, deadline: Instant) -> Result<ServiceBuild> {
        match self
            .client
            .call(self.peer.hub(), Command::GetBuild {}, deadline)
            .await
        {
            Ok(Reply::Build { build }) => Ok(build),
            Err(ClientError::Unsupported | ClientError::Remote(PublicError::Unsupported)) => {
                Err(BootstrapError::UpgradeRequired.into())
            }
            Err(error) => Err(error.into()),
            _ => bail!(UNEXPECTED_REPLY),
        }
    }
    pub async fn snapshot(&mut self, deadline: Instant) -> Result<ConfigurationSnapshot> {
        match self
            .client
            .call(self.peer.hub(), Command::GetConfiguration {}, deadline)
            .await?
        {
            Reply::Configuration { snapshot } => {
                ensure!(snapshot.state.identity == self.peer, PIN_MISMATCH);
                Ok(snapshot)
            }
            _ => bail!(UNEXPECTED_REPLY),
        }
    }
    pub async fn configure(
        &mut self,
        change: ConfigurationChange,
        deadline: Instant,
    ) -> std::result::Result<ConfigurationSnapshot, ClientError> {
        match self
            .client
            .call(self.peer.hub(), Command::Configure { change }, deadline)
            .await?
        {
            Reply::Configuration { snapshot } if snapshot.state.identity == self.peer => {
                Ok(snapshot)
            }
            _ => Err(ClientError::OutcomeUnknownResponse(
                PublicError::OriginMismatch,
            )),
        }
    }
}
