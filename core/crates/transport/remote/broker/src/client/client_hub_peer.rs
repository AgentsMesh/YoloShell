use super::*;
use yolo_broker_hub_protocol::{
    BrokerHubConnectionBinding, BrokerHubError, BrokerHubOpenRequest, BrokerHubProfile,
};
use yolo_broker_tunnel::BrokerHubTunnel;

impl BrokerClient {
    /// Preparation is explicitly authorized for this immutable Hub-only profile revision.
    pub async fn prepare_hub_transport(
        &self,
        expected: BrokerHubProfile,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerHubConnectionBinding, BrokerClientError> {
        self.prepare_hub_transport_inner(expected, None, deadline)
            .await
    }

    pub(in crate::remote_broker) async fn prepare_hub_transport_at(
        &self,
        expected: BrokerHubProfile,
        socket: SocketIdentity,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerHubConnectionBinding, BrokerClientError> {
        self.prepare_hub_transport_inner(expected, Some(socket), deadline)
            .await
    }

    async fn prepare_hub_transport_inner(
        &self,
        expected: BrokerHubProfile,
        socket: Option<SocketIdentity>,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerHubConnectionBinding, BrokerClientError> {
        expected.validate()?;
        let mut connection = self.connect(socket, deadline).await?;
        let response = request_on(
            connection.stream(),
            BrokerRequest::PrepareHubTransport(expected.clone()),
            deadline,
        )
        .await?;
        let BrokerResult::HubTransport(binding) = response else {
            return Err(BrokerClientError::UnexpectedResult);
        };
        binding.validate()?;
        if binding.profile != expected {
            return Err(BrokerHubError::ProfileMismatch.into());
        }
        connection.verify_path()?;
        Ok(binding)
    }

    /// Opens exactly this connection generation. Never reconnects or replays the open.
    pub async fn open_hub_peer(
        &self,
        request: BrokerHubOpenRequest,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerHubTunnel, BrokerClientError> {
        request.validate()?;
        let mut connection = self.connect(None, deadline).await?;
        let response = request_on(
            connection.stream(),
            BrokerRequest::OpenHubPeer(request.clone()),
            deadline,
        )
        .await?;
        let BrokerResult::HubPeer(opened) = response else {
            return Err(BrokerClientError::UnexpectedResult);
        };
        opened.validate(&request)?;
        let tunnel =
            BrokerHubTunnel::new(connection.into_stream()?, opened).map_err(ProtocolError::Io)?;
        if tokio::time::Instant::now() >= deadline {
            return Err(BrokerClientError::Deadline);
        }
        Ok(tunnel)
    }
}
