use super::deadline::ReconcileDeadline;
use super::{BrokerSpawner, SpawnAuthority};
use crate::start_authority::StartAuthority;
use crate::{BrokerTransition, StartRequest};
use anyhow::Result;
use yolo_remote_broker::{
    BROKER_READY_POLL_INTERVAL, BrokerClient, BrokerConfigGeneration, BrokerPaths,
    BrokerReconcileGuard, BrokerRetirement, authorize_broker_launch, broker_instance_available,
};

const LAST_OBSERVATION_PREFIX: &str = "; last broker observation failed: ";
const LAST_EXIT_PREFIX: &str = "; last admitted broker exited early: ";
const LAST_SPAWN_PREFIX: &str = "; last spawn failed: ";
const ADOPTION_CUSTODY_LOST: &str = "pending broker adoption lost its exact launch custody";

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    request: &StartRequest,
    spawner: &dyn BrokerSpawner,
    paths: &BrokerPaths,
    client: &BrokerClient,
    generation: &BrokerConfigGeneration,
    single_flight: &BrokerReconcileGuard,
    deadline: &ReconcileDeadline,
    authority: Option<&dyn StartAuthority>,
    spawns: &mut SpawnAuthority,
) -> Result<yolo_remote_broker::BrokerStatus> {
    let mut retirement = None;
    let mut last_observation = None;
    while tokio::time::Instant::now() < deadline.active() {
        spawns.refresh()?;
        verify_authorities(authority, single_flight)?;
        let observed = BrokerRetirement::capture(single_flight)?;
        match observed
            .transition(
                client,
                generation.revision(),
                single_flight,
                deadline.active(),
            )
            .await
        {
            Ok(transition) => {
                last_observation = None;
                single_flight.verify()?;
                match transition {
                    BrokerTransition::Current(status) => {
                        verify_authorities(authority, single_flight)?;
                        let mut ready = super::readiness::require(
                            &observed,
                            client,
                            single_flight,
                            &status,
                            generation,
                            deadline.active(),
                        )
                        .await?;
                        verify_authorities(authority, single_flight)?;
                        if let Some(authority) = authority {
                            authority.commit(&ready)?;
                        }
                        spawns.commit(&ready)?;
                        ready.adopted = true;
                        if let Some(authority) = authority {
                            authority.finished(&ready)?;
                        }
                        return Ok(ready);
                    }
                    BrokerTransition::Pending(status) => {
                        verify_authorities(authority, single_flight)?;
                        if spawns.owns(&status) {
                            let mut ready = super::readiness::require(
                                &observed,
                                client,
                                single_flight,
                                &status,
                                generation,
                                deadline.active(),
                            )
                            .await?;
                            verify_authorities(authority, single_flight)?;
                            if let Some(authority) = authority {
                                authority.commit(&ready)?;
                            }
                            if spawns.adopt(&ready)? {
                                ready.adopted = true;
                                if let Some(authority) = authority {
                                    authority.finished(&ready)?;
                                }
                                return Ok(ready);
                            }
                            anyhow::bail!(ADOPTION_CUSTODY_LOST);
                        }
                    }
                    BrokerTransition::Shutdown => retirement = Some(observed),
                }
            }
            Err(error) => {
                last_observation = Some(format!("{error:#}"));
                single_flight.verify()?;
                let ready = spawn_ready(&retirement, paths, single_flight).await?;
                let admission = tokio::time::Instant::now();
                if ready && admission < deadline.active() && spawns.can_admit(admission) {
                    verify_authorities(authority, single_flight)?;
                    let launch_authority =
                        authorize_broker_launch(paths, single_flight, deadline.active()).await?;
                    if tokio::time::Instant::now() >= deadline.active() {
                        continue;
                    }
                    let launch_admission = deadline.prepare_launch()?;
                    let launch = super::launch::options(
                        request,
                        generation,
                        paths,
                        launch_authority,
                        launch_admission.deadline(),
                    )?;
                    single_flight.verify()?;
                    launch_admission.verify()?;
                    spawns.begin()?;
                    let result = spawner.spawn(&launch, generation.credential_binding_nonce());
                    if result.is_err() {
                        single_flight.verify()?;
                    }
                    spawns.finish(result)?;
                    single_flight.verify()?;
                    retirement = None;
                }
            }
        }
        deadline
            .wait_for_next_attempt(BROKER_READY_POLL_INTERVAL)
            .await;
    }
    timeout_error(generation, client, spawns, last_observation)
}

fn verify_authorities(
    authority: Option<&dyn StartAuthority>,
    single_flight: &BrokerReconcileGuard,
) -> Result<()> {
    if let Some(authority) = authority {
        authority.verify()?;
    }
    single_flight.verify()
}

async fn spawn_ready(
    retirement: &Option<BrokerRetirement>,
    paths: &BrokerPaths,
    single_flight: &BrokerReconcileGuard,
) -> Result<bool> {
    match retirement {
        Some(observed) => observed.retire_if_unlocked(paths, single_flight).await,
        None => {
            single_flight.verify()?;
            let available = broker_instance_available(paths)?;
            single_flight.verify()?;
            Ok(available)
        }
    }
}

fn timeout_error(
    generation: &BrokerConfigGeneration,
    client: &BrokerClient,
    spawns: &SpawnAuthority,
    last_observation: Option<String>,
) -> Result<yolo_remote_broker::BrokerStatus> {
    let observation = last_observation
        .map(|error| format!("{LAST_OBSERVATION_PREFIX}{error}"))
        .unwrap_or_default();
    let exit = spawns
        .last_exit_error()
        .map(|error| format!("{LAST_EXIT_PREFIX}{error}"))
        .unwrap_or_default();
    let spawn = spawns
        .last_spawn_error()
        .map(|error| format!("{LAST_SPAWN_PREFIX}{error}"))
        .unwrap_or_default();
    anyhow::bail!(
        "broker did not reach configuration revision {} at {}; spawn attempts: {}{}{}{}",
        generation.revision(),
        client.socket_path().display(),
        spawns.attempts(),
        observation,
        exit,
        spawn,
    )
}
