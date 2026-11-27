use crate::{Engine, broker_directory::BrokerDirectory, constants::*};
use anyhow::{Result, ensure};
use yolo_hub_dispatch::Operation;
use yolo_hub_management_protocol::{
    CONFIGURATION_REVISION_STEP, ConfigurationChange, ConfigurationSnapshot,
};
use yolo_hub_remote_setup_protocol::{
    RemoteSetupOutcome, SetupAction, configuration_contains_link, plan_link,
};
use yolo_hub_remote_setup_store::{SetupOperation, SetupPhase};
use yolo_hub_rpc_protocol::{Command, Reply, Request};
use yolo_remote_provisioning_protocol::ProvisionStatus;

impl Engine {
    pub async fn finish_route(
        &self,
        operation: &mut SetupOperation,
        provision: ProvisionStatus,
        action: SetupAction,
        broker: &BrokerDirectory,
    ) -> Result<RemoteSetupOutcome> {
        broker.verify()?;
        let broker_directory = broker.identity();
        let record = operation.record().clone();
        match &record.phase {
            SetupPhase::Routing { change, .. } | SetupPhase::Complete { change, .. } => {
                let current = self.configuration().await?;
                operation.verify()?;
                broker.verify()?;
                let confirmed = match change {
                    Some(change) => confirms(&current, change),
                    None => current == record.baseline,
                };
                return Ok(
                    if confirmed && configuration_contains_link(&current, &provision)? {
                        RemoteSetupOutcome::Linked {
                            provision,
                            configuration: current,
                        }
                    } else {
                        RemoteSetupOutcome::Conflict { provision }
                    },
                );
            }
            _ => {}
        }
        if action == SetupAction::Inspect {
            return Ok(RemoteSetupOutcome::Remote { provision });
        }
        let current = self.configuration().await?;
        broker.verify()?;
        if current != record.baseline {
            return Ok(RemoteSetupOutcome::Conflict { provision });
        }
        let change = match plan_link(&record.baseline, &record.request, &provision) {
            Ok(change) => change,
            Err(_) => return Ok(RemoteSetupOutcome::Conflict { provision }),
        };
        operation.transition(SetupPhase::Routing {
            broker_directory,
            change: change.clone(),
            provision: provision.clone(),
        })?;
        self.profiles.verify_registration(&record.registration)?;
        let applied = if let Some(change) = &change {
            let request = Request::new(
                self.state.hub_id(),
                Command::Configure {
                    change: change.clone(),
                },
            )?;
            match self.inner.dispatch(request).await {
                Ok(Operation::Reply(Reply::Configuration { snapshot }))
                    if confirms(&snapshot, change) =>
                {
                    snapshot
                }
                _ => {
                    let observed = self.configuration().await?;
                    if !confirms(&observed, change) {
                        return Ok(RemoteSetupOutcome::Unknown {});
                    }
                    observed
                }
            }
        } else {
            current
        };
        operation.verify()?;
        broker.verify()?;
        self.profiles.verify_registration(&record.registration)?;
        ensure!(
            configuration_contains_link(&applied, &provision)?,
            UNEXPECTED
        );
        operation.transition(SetupPhase::Complete {
            broker_directory,
            change,
            provision: provision.clone(),
        })?;
        Ok(RemoteSetupOutcome::Linked {
            provision,
            configuration: applied,
        })
    }
}

pub(crate) fn confirms(snapshot: &ConfigurationSnapshot, change: &ConfigurationChange) -> bool {
    snapshot.applied
        && snapshot.state.last_change == change.request_id
        && change
            .expected_revision
            .checked_add(CONFIGURATION_REVISION_STEP)
            == Some(snapshot.state.revision)
        && snapshot.state.configuration == change.configuration
}
