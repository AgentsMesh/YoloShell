use super::super::{BrokerConfig, BrokerConnectionFactory};
use super::BrokerConnectionPool;
use crate::remote::RemoteError;
use std::sync::Arc;
use yolo_broker_hub_protocol::{BrokerHubError, BrokerHubProfile};

impl BrokerConnectionPool {
    pub(in crate::remote_broker) fn configured(
        config: &BrokerConfig,
        factory: Arc<dyn BrokerConnectionFactory>,
    ) -> Self {
        let mut pool = Self::new(
            config.host_id(),
            config.revision(),
            config.runtime().clone(),
            factory,
        );
        pool.hub_transport = config.hub_transport();
        if config.hub_transport() && config.artifact.is_none() {
            pool.hub_profile = Some(BrokerHubProfile {
                host_id: config.host_id().into(),
                profile_authority_sha256: config.profile_authority_sha256().into(),
                config_revision: config.revision().into(),
            });
        }
        pool
    }
    pub(in crate::remote_broker) fn require_runtime_mode(&self) -> Result<(), RemoteError> {
        if self.hub_transport {
            return Err(RemoteError::InvalidRuntime(
                BrokerHubError::Mode.to_string(),
            ));
        }
        Ok(())
    }
    pub(in crate::remote_broker) fn require_hub_profile(
        &self,
        expected: &BrokerHubProfile,
    ) -> anyhow::Result<()> {
        expected.validate()?;
        if !self.hub_transport {
            return Err(BrokerHubError::Mode.into());
        }
        let actual = self.hub_profile.as_ref().ok_or(BrokerHubError::Artifact)?;
        if actual != expected {
            return Err(BrokerHubError::ProfileMismatch.into());
        }
        Ok(())
    }
    pub(in crate::remote_broker) fn hub_capabilities(&self) -> Vec<String> {
        let mut capabilities = super::super::BrokerStatus::capabilities();
        if self.hub_profile.is_some() {
            capabilities
                .retain(|value| value != super::super::constants::CONNECTION_BOOTSTRAP_CAPABILITY);
            capabilities.push(yolo_broker_hub_protocol::HUB_TRANSPORT_CAPABILITY.into());
            capabilities.push(yolo_remote_provisioning_protocol::PROVISION_CAPABILITY.into());
        }
        capabilities
    }
}
