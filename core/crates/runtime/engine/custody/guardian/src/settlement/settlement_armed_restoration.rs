use super::admission::require_runtime;
use super::{ARMED_DETAIL, RECONCILIATION_DETAIL};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Result, ensure};
use std::cell::Cell;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

pub(super) fn restore(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    armed: &CustodyDurableRecord,
    armed_identity: FileIdentity,
    prepared: &CustodyDurableRecord,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
) -> Result<FileIdentity> {
    restore_observed(
        guard,
        paths,
        armed,
        armed_identity,
        prepared,
        runtime,
        runtime_identity,
        || {},
    )
}

#[cfg(test)]
pub(super) fn restore_with_commit_observer<F>(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    armed: &CustodyDurableRecord,
    armed_identity: FileIdentity,
    prepared: &CustodyDurableRecord,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
    after_commit: F,
) -> Result<FileIdentity>
where
    F: FnMut(),
{
    restore_observed(
        guard,
        paths,
        armed,
        armed_identity,
        prepared,
        runtime,
        runtime_identity,
        after_commit,
    )
}

#[allow(clippy::too_many_arguments)]
fn restore_observed<F>(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    armed: &CustodyDurableRecord,
    armed_identity: FileIdentity,
    prepared: &CustodyDurableRecord,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
    mut after_commit: F,
) -> Result<FileIdentity>
where
    F: FnMut(),
{
    let path = paths.pty_custody(armed.runtime.terminal_id);
    let committed = Cell::new(None);
    let result = crate::pty_custody::replace_exact_record_observed_with_fence(
        guard,
        &path,
        armed,
        armed_identity,
        prepared,
        |before_rename| {
            let installed: Result<(CustodyDurableRecord, FileIdentity)> =
                guard.namespace().read_json_with_identity(&path);
            if let Ok((record, identity)) = installed
                && record == *prepared
                && identity.same_content_version(before_rename)
            {
                committed.set(Some(identity));
                after_commit();
            }
        },
        |_| require_runtime(guard, paths, runtime, runtime_identity),
    );
    match result {
        Ok(identity) => Ok(identity),
        Err(original) => {
            let Some(identity) = committed.get() else {
                return Err(original);
            };
            reconcile(guard, paths, prepared, identity, runtime, runtime_identity)
                .map(|()| identity)
                .map_err(|error| original.context(format!("{RECONCILIATION_DETAIL}: {error:#}")))
        }
    }
}

fn reconcile(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    prepared: &CustodyDurableRecord,
    identity: FileIdentity,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
) -> Result<()> {
    require_restored(guard, paths, prepared, identity, runtime, runtime_identity)?;
    guard
        .namespace()
        .file(&paths.pty_custody(prepared.runtime.terminal_id))?
        .sync_parent()?;
    require_restored(guard, paths, prepared, identity, runtime, runtime_identity)
}

fn require_restored(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    prepared: &CustodyDurableRecord,
    identity: FileIdentity,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    let path = paths.pty_custody(prepared.runtime.terminal_id);
    let current = guard.namespace().read_json_with_identity(&path)?;
    ensure!(current == (prepared.clone(), identity), ARMED_DETAIL);
    require_runtime(guard, paths, runtime, runtime_identity)?;
    guard.require_no_pending_intent()
}
