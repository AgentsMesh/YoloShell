use super::*;
use yolo_broker_hub_protocol::{BrokerHubConnectionBinding, BrokerHubProfile};

impl BrokerRetirement {
    pub async fn prepare_hub_transport(
        &self,
        client: &BrokerClient,
        guard: &BrokerReconcileGuard,
        expected: BrokerHubProfile,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerHubConnectionBinding> {
        guard.verify()?;
        let socket = self
            .management
            .context(BROKER_MANAGEMENT_ENDPOINT_MISSING)?;
        let binding = client
            .prepare_hub_transport_at(expected, socket, deadline)
            .await;
        guard.verify()?;
        Ok(binding?)
    }
}
