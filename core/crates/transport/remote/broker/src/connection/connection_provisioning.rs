use super::BrokerConnectionPool;
use crate::remote_broker::{BrokerPaths, provisioning_rpc::Action};
use anyhow::{Result, ensure};
use tokio::time::Instant;
use yolo_broker_hub_protocol::BrokerHubError;
use yolo_remote_provisioning::{ProvisionGuard, RemoteProvisioningService};
use yolo_remote_provisioning_protocol::{BrokerProvisionCall, ProvisionOutcome, ProvisionStatus};

impl BrokerConnectionPool {
    pub(in crate::remote_broker) fn retain_provisioning_paths(&mut self, paths: &BrokerPaths) {
        self.provisioning_paths = Some(paths.clone());
    }
    pub(in crate::remote_broker) async fn provision_hub(
        &self,
        call: &BrokerProvisionCall,
        action: Action,
        deadline: Instant,
        admission: &dyn ProvisionGuard,
    ) -> Result<ProvisionStatus> {
        call.validate()?;
        admission.verify()?;
        self.require_hub_profile(&call.binding.profile)?;
        let observed = self
            .cached_connection()
            .await
            .ok_or(BrokerHubError::StaleBinding)?;
        ensure!(
            self.hub_binding(call.binding.profile.clone(), &observed) == call.binding,
            BrokerHubError::StaleBinding
        );
        let paths = self
            .provisioning_paths
            .as_ref()
            .ok_or(BrokerHubError::Mode)?;
        let verify = || {
            admission.verify()?;
            self.require_hub_profile(&call.binding.profile)?;
            paths.verify_state()?;
            ensure!(!self.is_retired(&observed), BrokerHubError::StaleBinding);
            Ok(())
        };
        verify()?;
        let parent = paths.state_directory()?;
        let service = match action {
            Action::Provision => Some(RemoteProvisioningService::open(
                parent,
                call.binding.profile.clone(),
            )?),
            Action::Recover | Action::Advance => {
                RemoteProvisioningService::open_existing(parent, call.binding.profile.clone())?
            }
        };
        let Some(service) = service else {
            return ProvisionStatus::new(call.request.clone(), ProvisionOutcome::Absent {});
        };
        let connection = observed.connection().as_ref();
        let result = match action {
            Action::Provision => {
                service
                    .provision(connection, &verify, &call.request, deadline)
                    .await
            }
            Action::Recover => {
                service
                    .recover(connection, &verify, &call.request, deadline)
                    .await
            }
            Action::Advance => {
                service
                    .continue_known_stage(connection, &verify, &call.request, deadline)
                    .await
            }
        };
        verify()?;
        result
    }
}
