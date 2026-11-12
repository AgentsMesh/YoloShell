use super::CodexHookArtifacts;
use anyhow::{Context, Result, ensure};

#[path = "admission_orphan_recovery.rs"]
mod recovery;
#[path = "admission_orphan_shape.rs"]
mod shape;

const ORPHAN_CORRUPT: &str = "Codex pre-admission artifact is corrupt";

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum StagingResidue {
    Active,
    Isolated,
}

pub(super) fn is_pre_admission_orphan(artifacts: &CodexHookArtifacts) -> Result<bool> {
    let invocation = crate::paths::directory::PrivateDirectory::open(artifacts.directory())
        .context(ORPHAN_CORRUPT)?;
    let path = artifacts
        .directory()
        .join(super::admission_storage::CODEX_ADMISSION_FILE_NAME);
    let admission = crate::paths::private_file::PrivateFileEntry::from_directory(
        invocation.try_clone()?,
        &path,
    )?;
    admission
        .verify_expected(None, crate::paths::MAX_DURABLE_JSON_BYTES)
        .context(ORPHAN_CORRUPT)?;
    let staging = admission.exclusive_create_staging_entry()?;
    let isolation = staging.unlink_isolation_entry()?;
    let Some(initial) = shape::inspect(
        &invocation,
        artifacts,
        staging.file_name(),
        isolation.file_name(),
    )?
    else {
        return Ok(false);
    };
    let Some(residue) = initial.residue else {
        admission
            .verify_expected(None, crate::paths::MAX_DURABLE_JSON_BYTES)
            .context(ORPHAN_CORRUPT)?;
        invocation.verify_path().context(ORPHAN_CORRUPT)?;
        ensure!(invocation.revision()? == initial.revision, ORPHAN_CORRUPT);
        return Ok(true);
    };
    let lock = initial.lock_identity.context(ORPHAN_CORRUPT)?;
    let writer = recovery::reclaim(&invocation, artifacts, &admission, &staging, residue, lock)?;
    let settled = shape::inspect(
        &invocation,
        artifacts,
        staging.file_name(),
        isolation.file_name(),
    )?
    .context(ORPHAN_CORRUPT)?;
    ensure!(
        settled.residue.is_none()
            && settled.saw_intake == initial.saw_intake
            && settled.lock_identity == Some(lock),
        ORPHAN_CORRUPT
    );
    admission
        .verify_expected(None, crate::paths::MAX_DURABLE_JSON_BYTES)
        .context(ORPHAN_CORRUPT)?;
    writer.verify().context(ORPHAN_CORRUPT)?;
    invocation.verify_path().context(ORPHAN_CORRUPT)?;
    ensure!(invocation.revision()? == settled.revision, ORPHAN_CORRUPT);
    Ok(true)
}

#[cfg(test)]
pub(super) fn set_before_stage_reclaim_hook(hook: impl FnOnce() + 'static) {
    recovery::set_before_reclaim_hook(hook);
}
