use crate::model::{ExitReason, ExitRecord, RuntimeRecord};
use crate::paths::identity::FileIdentity;
use crate::worker::{AgentProjectionShutdown, WorkerState, lock_recover};
use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use std::path::Path;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const EXIT_INVALID: &str = "revoked session has no exact durable Lost exit";
const RUNTIME_INVALID: &str = "revoked session runtime authority changed before shutdown";
const SUCCESSOR_PENDING: &str = "revoked session has a pending successor launch";

pub(super) async fn settle(
    state: &WorkerState,
    disposition: AgentProjectionShutdown,
) -> Result<bool> {
    match disposition {
        AgentProjectionShutdown::Invalidate => super::publication::invalidate(state).await,
        AgentProjectionShutdown::ManifestRevoked(manifest) => {
            confirm_stale_projection(state, manifest).await
        }
    }
}

async fn confirm_stale_projection(
    state: &WorkerState,
    manifest: crate::worker::RemovedTerminalManifest,
) -> Result<bool> {
    let _gate = state.agent_generation_gate.lock().await;
    let guard = SessionDurableMutationGuard::acquire_participant(
        &state.paths,
        state.namespace.lease(),
        state.terminal_id(),
    )?;
    guard.require_no_pending_intent()?;
    manifest.verify_missing(state)?;
    let expected = lock_recover(&state.record).clone();
    let runtime_path = state.paths.runtime_record(state.terminal_id());
    let (runtime, runtime_identity): (RuntimeRecord, FileIdentity) = guard
        .read_optional_with_identity(&runtime_path)?
        .context(RUNTIME_INVALID)?;
    ensure!(runtime == expected, RUNTIME_INVALID);
    let launch_path = state.paths.worker_launch(state.terminal_id());
    ensure!(
        guard
            .read_optional_with_identity::<crate::model::WorkerLaunchSpec>(&launch_path)?
            .is_none(),
        SUCCESSOR_PENDING
    );
    let observed = lock_recover(&state.exit).clone().context(EXIT_INVALID)?;
    let exit_path = state.paths.exit_record(state.terminal_id());
    let (durable, exit_identity): (ExitRecord, FileIdentity) = guard
        .namespace()
        .read_json_with_identity(&exit_path)
        .context(EXIT_INVALID)?;
    ensure!(
        durable == observed
            && durable.runtime_epoch == expected.runtime_epoch
            && durable.reason == ExitReason::Lost,
        EXIT_INVALID
    );
    verify_exact(
        &guard,
        &runtime_path,
        &runtime,
        runtime_identity,
        RUNTIME_INVALID,
    )?;
    ensure!(
        guard
            .read_optional_with_identity::<crate::model::WorkerLaunchSpec>(&launch_path)?
            .is_none(),
        SUCCESSOR_PENDING
    );
    let (current_exit, current_exit_identity): (ExitRecord, FileIdentity) = guard
        .namespace()
        .read_json_with_identity(&exit_path)
        .context(EXIT_INVALID)?;
    ensure!(
        current_exit == durable && current_exit_identity == exit_identity,
        EXIT_INVALID
    );
    guard.require_no_pending_intent()?;
    manifest.verify_missing(state)?;
    Ok(false)
}

fn verify_exact<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &T,
    identity: FileIdentity,
    detail: &'static str,
) -> Result<()>
where
    T: DeserializeOwned + PartialEq,
{
    let (current, current_identity) = guard
        .read_optional_with_identity::<T>(path)?
        .context(detail)?;
    ensure!(current == *expected && current_identity == identity, detail);
    Ok(())
}
