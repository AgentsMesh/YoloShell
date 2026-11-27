use super::ManifestFence;
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Result, ensure};
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const CUSTODY_DETAIL: &str = "PTY custody promotion predecessor changed";
const RUNTIME_DETAIL: &str = "PTY custody promotion runtime changed";

pub(super) fn require_custody(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) = guard
        .namespace()
        .read_json_with_identity(&paths.pty_custody(expected.runtime.terminal_id))?;
    ensure!(
        expected.is_valid()
            && expected.armed_lease().is_some()
            && current == (expected.clone(), expected_identity),
        CUSTODY_DETAIL
    );
    Ok(())
}

pub(super) fn require_existing(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    custody: &CustodyDurableRecord,
    custody_identity: FileIdentity,
    manifest: &ManifestFence,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    require_runtime(guard, paths, runtime, runtime_identity, true)?;
    require_custody(guard, paths, custody, custody_identity)?;
    manifest.require_exact(guard, paths)?;
    guard.require_no_pending_intent()
}

pub(super) fn require_published(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    custody: &CustodyDurableRecord,
    custody_identity: FileIdentity,
    manifest: &ManifestFence,
    runtime: &RuntimeRecord,
    committed_identity: FileIdentity,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    require_runtime(guard, paths, runtime, committed_identity, false)?;
    require_custody(guard, paths, custody, custody_identity)?;
    manifest.require_exact(guard, paths)?;
    guard.require_no_pending_intent()
}

pub(super) fn reconcile_published(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    custody: &CustodyDurableRecord,
    custody_identity: FileIdentity,
    manifest: &ManifestFence,
    runtime: &RuntimeRecord,
    committed_identity: FileIdentity,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    require_custody(guard, paths, custody, custody_identity)?;
    manifest.require_exact(guard, paths)?;
    let path = paths.runtime_record(runtime.terminal_id);
    guard.namespace().file(&path)?.sync_parent()?;
    guard.require_no_pending_intent()?;
    require_runtime(guard, paths, runtime, committed_identity, false)?;
    require_custody(guard, paths, custody, custody_identity)?;
    manifest.require_exact(guard, paths)?;
    guard.require_no_pending_intent()
}

fn require_runtime(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    expected: &RuntimeRecord,
    expected_identity: FileIdentity,
    unchanged: bool,
) -> Result<()> {
    let current: (RuntimeRecord, FileIdentity) = guard
        .namespace()
        .read_json_with_identity(&paths.runtime_record(expected.terminal_id))?;
    let identity_matches = if unchanged {
        current.1 == expected_identity
    } else {
        current.1.same_content_version(expected_identity)
    };
    ensure!(current.0 == *expected && identity_matches, RUNTIME_DETAIL);
    Ok(())
}
