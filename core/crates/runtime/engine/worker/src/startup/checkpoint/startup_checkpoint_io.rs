use super::super::{Paths, WorkerNamespace};
use super::constants::*;
use super::{StartupCheckpointEvidence, validation};
use crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::Result;
use std::io::ErrorKind;

pub(crate) fn begin_after_lifecycle_lock(
    namespace: &WorkerNamespace,
    paths: &Paths,
    evidence: &StartupCheckpointEvidence,
) -> Result<(
    StartupCheckpointEvidence,
    FileIdentity,
    Option<super::StartupPredecessorAuthority>,
)> {
    validation::validate(evidence)?;
    let mutation = super::mutation::CheckpointMutation::acquire_after_lifecycle_lock(
        paths,
        namespace.lease(),
        evidence.terminal_id,
    )?;
    if let Some((stored, identity)) = mutation.read_optional()? {
        let launch = super::fence::from_evidence(evidence);
        anyhow::ensure!(
            stored.status == super::StartupStatus::HostRetired && stored.successor == Some(launch),
            STALE_ERROR
        );
        let (predecessor_authority, custody_rollover) =
            super::custody_authority::capture(&mutation, paths, &stored, launch)?;
        let mut active = evidence.clone();
        active.predecessor_runtime_sha256 = stored.predecessor_runtime_sha256;
        active.pending_pty_custody_rollover = custody_rollover;
        let installed_identity = mutation.write(Some(identity), &active)?;
        let predecessor_authority = predecessor_authority.map(|mut authority| {
            authority.checkpoint_identity = installed_identity;
            authority
        });
        return Ok((active, installed_identity, predecessor_authority));
    }
    Ok((evidence.clone(), mutation.write(None, evidence)?, None))
}

pub(crate) fn replace(
    namespace: &WorkerNamespace,
    paths: &Paths,
    current: &StartupCheckpointEvidence,
    replacement: &StartupCheckpointEvidence,
    expected_identity: Option<FileIdentity>,
) -> Result<FileIdentity> {
    replace_with(
        namespace,
        paths,
        current,
        replacement,
        expected_identity,
        |_| Ok(()),
    )
}

pub(crate) fn replace_with<F>(
    namespace: &WorkerNamespace,
    paths: &Paths,
    current: &StartupCheckpointEvidence,
    replacement: &StartupCheckpointEvidence,
    expected_identity: Option<FileIdentity>,
    before_cas: F,
) -> Result<FileIdentity>
where
    F: FnOnce(&super::mutation::CheckpointMutation<'_>) -> Result<()>,
{
    let mutation = super::mutation::CheckpointMutation::acquire(
        paths,
        namespace.lease(),
        current.terminal_id,
    )?;
    let (stored, identity) = mutation
        .read_optional()?
        .ok_or_else(|| anyhow::anyhow!(EVIDENCE_MISSING_ERROR))?;
    anyhow::ensure!(stored == *current, STALE_ERROR);
    if let Some(expected_identity) = expected_identity {
        anyhow::ensure!(identity == expected_identity, STALE_ERROR);
    }
    before_cas(&mutation)?;
    mutation.require_no_pending_intent()?;
    mutation.write(Some(identity), replacement)
}

#[cfg(test)]
pub(crate) fn replace_with_test_hook<F>(
    namespace: &WorkerNamespace,
    paths: &Paths,
    current: &StartupCheckpointEvidence,
    replacement: &StartupCheckpointEvidence,
    before_cas: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    replace_with(namespace, paths, current, replacement, None, |_| {
        before_cas()
    })
    .map(|_| ())
}

#[cfg(test)]
pub(crate) fn begin_with_test_hook<F>(
    namespace: &WorkerNamespace,
    paths: &Paths,
    evidence: &StartupCheckpointEvidence,
    before_cas: F,
) -> Result<Option<super::StartupPredecessorAuthority>>
where
    F: FnOnce() -> Result<()>,
{
    validation::validate(evidence)?;
    let mutation = super::mutation::CheckpointMutation::acquire_after_lifecycle_lock(
        paths,
        namespace.lease(),
        evidence.terminal_id,
    )?;
    anyhow::ensure!(mutation.read_optional()?.is_none(), STALE_ERROR);
    before_cas()?;
    mutation.require_no_pending_intent()?;
    mutation.write(None, evidence)?;
    Ok(None)
}

pub(crate) fn read<F>(
    entry: PrivateFileEntry,
    guard: F,
) -> Result<(StartupCheckpointEvidence, FileIdentity)>
where
    F: FnOnce() -> Result<()>,
{
    let opened = entry.open_read(WORKER_STARTUP_CHECKPOINT_MAX_BYTES)?;
    super::read_io::read_opened(entry, opened, guard)
}

pub(crate) fn read_optional<F>(
    entry: PrivateFileEntry,
    guard: F,
) -> Result<Option<(StartupCheckpointEvidence, FileIdentity)>>
where
    F: FnOnce() -> Result<()>,
{
    let opened = match entry.open_read(WORKER_STARTUP_CHECKPOINT_MAX_BYTES) {
        Ok(opened) => opened,
        Err(error) if crate::paths::private_file::io_kind(&error) == Some(ErrorKind::NotFound) => {
            guard()?;
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    super::read_io::read_opened(entry, opened, guard).map(Some)
}
