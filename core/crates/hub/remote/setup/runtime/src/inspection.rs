use crate::{Engine, broker_directory::BrokerDirectory, constants::*};
use anyhow::{Result, ensure};
use tokio::time::Instant;
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, configuration_contains_link};
use yolo_hub_remote_setup_store::{SetupObservation, SetupPhase};
use yolo_hub_ssh_profile_runtime::{ProfileReservation, profile_binding};
use yolo_remote_broker::{BrokerClient, BrokerPaths};
use yolo_remote_provisioning_protocol::{BrokerProvisionCall, ProvisionOutcome};

impl Engine {
    pub async fn validate_polled(
        &self,
        mut status: yolo_hub_remote_setup_protocol::RemoteSetupStatus,
        expected: &yolo_hub_remote_setup_protocol::RemoteSetupRequest,
    ) -> Result<yolo_hub_remote_setup_protocol::RemoteSetupStatus> {
        status.validate_for(expected)?;
        let observation =
            SetupObservation::observe(&self.root, expected.request.fields().request_id)?
                .ok_or_else(|| anyhow::anyhow!(JOURNAL_CHANGED))?;
        let record = observation.record();
        ensure!(
            record.owner == self.state.peer_identity() && record.request == *expected,
            INVALID
        );
        self.profiles.verify_registration(&record.registration)?;
        let broker = record
            .broker_directory()
            .map(|identity| BrokerDirectory::open(&record.registration, Some(identity)))
            .transpose()?;
        if let RemoteSetupOutcome::Linked {
            provision,
            configuration,
        } = &status.outcome
        {
            if self.configuration().await? != *configuration {
                status.outcome = RemoteSetupOutcome::Conflict {
                    provision: provision.clone(),
                };
            }
        }
        observation.verify()?;
        if let Some(broker) = broker {
            broker.verify()?;
        }
        Ok(status)
    }
    pub async fn inspect(
        &self,
        observation: SetupObservation,
        reservation: ProfileReservation,
        deadline: Instant,
    ) -> Result<RemoteSetupOutcome> {
        observation.verify()?;
        let record = observation.record();
        ensure!(record.registration == *reservation.registration(), INVALID);
        let Some(identity) = record.broker_directory() else {
            return Ok(RemoteSetupOutcome::Unknown {});
        };
        let broker = BrokerDirectory::open(&record.registration, Some(identity))?;
        self.profiles.verify_registration(&record.registration)?;
        let paths = BrokerPaths::open_existing(&record.registration.broker_state)?;
        let client = BrokerClient::new(paths.management_socket());
        let binding = client
            .prepare_hub_transport(profile_binding(&record.registration), deadline)
            .await?;
        let provision = client
            .recover_hub_provision(
                BrokerProvisionCall {
                    binding,
                    request: record.request.request.clone(),
                },
                deadline,
            )
            .await?;
        provision.validate_for(&record.request.request)?;
        observation.verify()?;
        broker.verify()?;
        self.profiles.verify_registration(&record.registration)?;
        let outcome = if matches!(provision.fields().outcome, ProvisionOutcome::Ready { .. }) {
            match &record.phase {
                SetupPhase::Routing { change, .. } | SetupPhase::Complete { change, .. } => {
                    let current = self.configuration().await?;
                    let confirmed = change
                        .as_ref()
                        .map_or(current == record.baseline, |change| {
                            crate::route::confirms(&current, change)
                        });
                    if confirmed && configuration_contains_link(&current, &provision)? {
                        RemoteSetupOutcome::Linked {
                            provision,
                            configuration: current,
                        }
                    } else {
                        RemoteSetupOutcome::Conflict { provision }
                    }
                }
                _ => RemoteSetupOutcome::Remote { provision },
            }
        } else {
            RemoteSetupOutcome::Remote { provision }
        };
        observation.verify()?;
        broker.verify()?;
        paths.verify_state()?;
        Ok(outcome)
    }
}
