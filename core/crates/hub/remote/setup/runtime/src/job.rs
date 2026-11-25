use crate::{Engine, broker_directory::BrokerDirectory, constants::*, diagnostics::Stage};
use anyhow::{Context, Result, ensure};
use tokio::time::Instant;
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, RemoteSetupRequest, SetupAction};
use yolo_hub_remote_setup_store::{SetupOperation, SetupPhase};
use yolo_hub_ssh_profile_protocol::SshBrokerProfile;
use yolo_hub_ssh_profile_runtime::{ProfileReservation, profile_binding};
use yolo_remote_broker::{BrokerClient, BrokerPaths};
use yolo_remote_provisioning_protocol::{BrokerProvisionCall, ProvisionOutcome};
use zeroize::Zeroizing;

pub(crate) fn profile_matches(
    registration: &SshBrokerProfile,
    request: &RemoteSetupRequest,
) -> Result<()> {
    ensure!(
        profile_binding(registration) == request.request.fields().profile,
        INVALID
    );
    Ok(())
}

impl Engine {
    pub async fn run(
        &self,
        mut operation: SetupOperation,
        reservation: ProfileReservation,
        action: SetupAction,
        credentials: Option<Zeroizing<Vec<u8>>>,
        deadline: Instant,
    ) -> Result<RemoteSetupOutcome> {
        operation.verify()?;
        if action == SetupAction::Inspect {
            let id = operation.record().request.request.fields().request_id;
            drop(operation);
            let observation =
                yolo_hub_remote_setup_store::SetupObservation::observe(&self.root, id)?
                    .ok_or_else(|| anyhow::anyhow!(JOURNAL_CHANGED))?;
            return self.inspect(observation, reservation, deadline).await;
        }
        let registration = reservation.registration();
        profile_matches(registration, &operation.record().request)?;
        self.profiles.verify_registration(registration)?;
        let prepared = matches!(operation.record().phase, SetupPhase::Prepared {});
        let expected = operation.record().broker_directory();
        let retained = expected
            .map(|identity| BrokerDirectory::open(registration, Some(identity)))
            .transpose()?;
        let binding = self
            .profiles
            .ensure_broker(&reservation, credentials, deadline)
            .await
            .context(Stage::BrokerStartup)?;
        let paths = BrokerPaths::open_existing(&registration.broker_state)?;
        let broker = if let Some(retained) = retained {
            retained.verify()?;
            retained
        } else {
            BrokerDirectory::open(registration, None)?
        };
        let broker_directory = broker.identity();
        let client = BrokerClient::new(paths.management_socket());
        let call = BrokerProvisionCall {
            binding,
            request: operation.record().request.request.clone(),
        };
        let mut provision = if prepared {
            operation.transition(SetupPhase::Provisioning { broker_directory })?;
            self.profiles.verify_registration(registration)?;
            client
                .provision_hub(call.clone(), deadline)
                .await
                .context(Stage::Provisioning)?
        } else {
            client
                .recover_hub_provision(call.clone(), deadline)
                .await
                .context(Stage::Provisioning)?
        };
        operation.verify()?;
        broker.verify()?;
        self.profiles.verify_registration(registration)?;
        if action == SetupAction::Continue
            && matches!(
                provision.fields().outcome,
                ProvisionOutcome::Published { .. }
            )
        {
            provision = client
                .advance_hub_provision(call, deadline)
                .await
                .context(Stage::Provisioning)?;
        }
        provision.validate_for(&operation.record().request.request)?;
        operation.verify()?;
        broker.verify()?;
        self.profiles.verify_registration(registration)?;
        if !matches!(provision.fields().outcome, ProvisionOutcome::Ready { .. }) {
            return Ok(RemoteSetupOutcome::Remote { provision });
        }
        self.finish_route(&mut operation, provision, action, &broker)
            .await
            .context(Stage::Routing)
    }
}
