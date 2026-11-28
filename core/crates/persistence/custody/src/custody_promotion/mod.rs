use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Context, Result, ensure};
use std::cell::Cell;
use yolo_paths::durable_mutation::{DurableMutationFencePhase, SessionDurableMutationGuard};

#[path = "manifest.rs"]
mod manifest;
#[path = "verification.rs"]
mod verification;

use manifest::ManifestFence;

const CUSTODY_DETAIL: &str = "PTY custody promotion predecessor changed";
const RUNTIME_DETAIL: &str = "PTY custody promotion runtime changed";
const RECONCILIATION_DETAIL: &str = "reconcile committed PTY custody runtime promotion";

pub fn publish_exact(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &CustodyDurableRecord,
    custody_identity: FileIdentity,
) -> Result<RuntimeRecord> {
    publish_exact_with_boundary(
        namespace,
        paths,
        custody,
        custody_identity,
        || {},
        || Ok(()),
    )
}

pub fn publish_exact_with_boundary<F, G>(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &CustodyDurableRecord,
    custody_identity: FileIdentity,
    after_commit: F,
    indeterminate_boundary: G,
) -> Result<RuntimeRecord>
where
    F: FnMut(),
    G: FnMut() -> Result<()>,
{
    let lease = custody.armed_lease().context(CUSTODY_DETAIL)?;
    let guard = SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace,
        custody.runtime.terminal_id,
    )?;
    guard.require_no_pending_intent()?;
    verification::require_custody(&guard, paths, custody, custody_identity)?;
    let manifest = ManifestFence::capture(&guard, paths)?;
    let runtime_path = paths.runtime_record(custody.runtime.terminal_id);
    let (current, current_identity): (RuntimeRecord, FileIdentity) =
        namespace.read_json_with_identity(&runtime_path)?;
    ensure!(
        custody.runtime.matches_record(&current)
            && current.controller_generation <= lease.controller_generation,
        RUNTIME_DETAIL
    );
    if current.controller_generation == lease.controller_generation {
        verification::require_existing(
            &guard,
            paths,
            custody,
            custody_identity,
            &manifest,
            &current,
            current_identity,
        )?;
        return Ok(current);
    }
    let mut promoted = current.clone();
    promoted.controller_generation = lease.controller_generation;
    promoted
        .validate_update_from(&current)
        .context(RUNTIME_DETAIL)?;
    publish_new(
        &guard,
        paths,
        custody,
        custody_identity,
        &manifest,
        &runtime_path,
        &current,
        current_identity,
        &promoted,
        after_commit,
        indeterminate_boundary,
    )?;
    Ok(promoted)
}

#[allow(clippy::too_many_arguments)]
fn publish_new<F, G>(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    custody: &CustodyDurableRecord,
    custody_identity: FileIdentity,
    manifest: &ManifestFence,
    runtime_path: &std::path::Path,
    predecessor: &RuntimeRecord,
    predecessor_identity: FileIdentity,
    promoted: &RuntimeRecord,
    mut after_commit: F,
    mut indeterminate_boundary: G,
) -> Result<()>
where
    F: FnMut(),
    G: FnMut() -> Result<()>,
{
    let committed = Cell::new(None);
    let result = guard
        .write_json_expected_with_fence_observer(
            runtime_path,
            Some(predecessor_identity),
            promoted,
            |phase| {
                verification::require_custody(guard, paths, custody, custody_identity)?;
                manifest.require_exact(guard, paths)?;
                require_runtime_phase(
                    guard,
                    runtime_path,
                    predecessor,
                    predecessor_identity,
                    promoted,
                    phase,
                )
            },
            |identity| {
                committed.set(Some(identity));
                after_commit();
            },
        )
        .and_then(|installed| {
            let observed = committed.get().context(RUNTIME_DETAIL)?;
            ensure!(installed.same_content_version(observed), RUNTIME_DETAIL);
            indeterminate_boundary()?;
            Ok(())
        });
    match result {
        Ok(()) => verification::require_published(
            guard,
            paths,
            custody,
            custody_identity,
            manifest,
            promoted,
            committed.get().context(RUNTIME_DETAIL)?,
        ),
        Err(original) => {
            let Some(identity) = committed.get() else {
                return Err(original);
            };
            verification::reconcile_published(
                guard,
                paths,
                custody,
                custody_identity,
                manifest,
                promoted,
                identity,
            )
            .map_err(|reconcile| {
                original.context(format!("{RECONCILIATION_DETAIL}: {reconcile:#}"))
            })
        }
    }
}

fn require_runtime_phase(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    predecessor: &RuntimeRecord,
    predecessor_identity: FileIdentity,
    promoted: &RuntimeRecord,
    phase: DurableMutationFencePhase,
) -> Result<()> {
    let current: (RuntimeRecord, FileIdentity) = guard.namespace().read_json_with_identity(path)?;
    match phase {
        DurableMutationFencePhase::Commit => {
            ensure!(
                current == (predecessor.clone(), predecessor_identity),
                RUNTIME_DETAIL
            )
        }
        DurableMutationFencePhase::Installed => ensure!(current.0 == *promoted, RUNTIME_DETAIL),
    }
    Ok(())
}
