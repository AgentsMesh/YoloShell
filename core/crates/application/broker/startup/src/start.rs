use super::{CredentialSource, ProfileAuthorityRequest, StartRequest};
use crate::pending::PendingSpawnHandle;
use crate::start_authority::StartAuthority;
use anyhow::Result;
use yolo_broker_child_custody::finish_after_termination_delivery;
use yolo_broker_profile_authority::StartAuthorityGuard;
use yolo_broker_supervision::DetachedSpawner;
use yolo_broker_supervision_state::ChildLaunch;
use yolo_remote_broker::{
    BROKER_READY_TIMEOUT, BROKER_SPAWN_RETRY_INTERVAL, BrokerClient, BrokerPaths,
    BrokerReconcileGuard, BrokerStatus, IdentityPassphrases,
    stage_broker_config_guarded_with_passphrases,
};
#[path = "start_attempt.rs"]
mod attempt;
#[path = "start_deadline.rs"]
mod deadline;
#[path = "start_hub.rs"]
mod hub;
#[path = "start_launch.rs"]
mod launch;
#[path = "start_readiness.rs"]
mod readiness;
#[path = "start_reconcile.rs"]
mod reconcile_loop;
#[path = "start_socket.rs"]
mod socket;
use attempt::SpawnAuthority;
pub use hub::run as run_hub;

pub async fn run(
    request: StartRequest,
    authority: ProfileAuthorityRequest,
    credentials: CredentialSource,
) -> Result<BrokerStatus> {
    let deadline = deadline::ReconcileDeadline::capture(BROKER_READY_TIMEOUT)?;
    let authority = StartAuthorityGuard::acquire(
        &authority.document,
        &authority.lock,
        &authority.expected_sha256,
    )?;
    let passphrases = match credentials {
        CredentialSource::None => IdentityPassphrases::none(),
        CredentialSource::PrivateStdin => IdentityPassphrases::read_stdin()?,
    };
    let spawner = DetachedSpawner::current(passphrases, credentials)?;
    let result = reconcile_with_passphrases(
        request,
        &spawner,
        &deadline,
        spawner.passphrases(),
        Some(&authority),
    )
    .await;
    finish_after_termination_delivery(result, deadline.exit_delivery())
}

pub(super) trait BrokerSpawner: Send + Sync {
    fn spawn(
        &self,
        launch: &ChildLaunch,
        credential_binding_nonce: Option<&str>,
    ) -> Result<PendingSpawnHandle>;
}

impl BrokerSpawner for DetachedSpawner {
    fn spawn(&self, launch: &ChildLaunch, nonce: Option<&str>) -> Result<PendingSpawnHandle> {
        DetachedSpawner::spawn(self, launch, nonce)
            .map(|pending| Box::new(pending) as PendingSpawnHandle)
    }
}

#[cfg(test)]
pub(super) async fn reconcile(
    request: StartRequest,
    spawner: &dyn BrokerSpawner,
    timeout: std::time::Duration,
) -> Result<BrokerStatus> {
    let deadline = deadline::ReconcileDeadline::capture(timeout)?;
    reconcile_with_passphrases(
        request,
        spawner,
        &deadline,
        &IdentityPassphrases::none(),
        None,
    )
    .await
}

async fn reconcile_with_passphrases(
    request: StartRequest,
    spawner: &dyn BrokerSpawner,
    deadline: &deadline::ReconcileDeadline,
    passphrases: &IdentityPassphrases,
    authority: Option<&dyn StartAuthority>,
) -> Result<BrokerStatus> {
    deadline.verify()?;
    if let Some(authority) = authority {
        authority.verify_request(&request)?;
    }
    let paths = BrokerPaths::new(&request.broker_state)?;
    let single_flight = BrokerReconcileGuard::acquire(&paths, deadline.active()).await?;
    deadline.verify()?;
    single_flight.verify()?;
    socket::validate_existing(&paths.runtime_socket())?;
    socket::validate_existing(&paths.management_socket())?;
    single_flight.verify()?;
    let client = BrokerClient::new(paths.management_socket());
    // A reconcile operation is bound to this immutable generation.  Re-reading the
    // source config here would let an older operation replace a newer request.
    single_flight.verify()?;
    let generation = stage_broker_config_guarded_with_passphrases(
        &request.config_source,
        &paths,
        &single_flight,
        passphrases,
    )?;
    deadline.verify()?;
    if let Some(authority) = authority {
        authority.verify_generation(&generation)?;
    }
    single_flight.verify()?;
    let mut spawns = SpawnAuthority::new(BROKER_SPAWN_RETRY_INTERVAL);
    let outcome = reconcile_loop::run(
        &request,
        spawner,
        &paths,
        &client,
        &generation,
        &single_flight,
        deadline,
        authority,
        &mut spawns,
    )
    .await;
    spawns.resolve(outcome)
}
