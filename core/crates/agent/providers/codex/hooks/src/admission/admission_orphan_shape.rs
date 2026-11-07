use super::super::CodexHookArtifacts;
use anyhow::{Context, Result, ensure};
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;

const ORPHAN_CORRUPT: &str = "Codex pre-admission artifact is corrupt";
const ENTRY_INCREMENT: usize = 1;
const WRITER_LOCK_ENTRY_COUNT: usize = 1;
const INTAKE_ENTRY_COUNT: usize = 1;
const STAGING_RESIDUE_ENTRY_COUNT: usize = 1;
const MAX_EXPECTED_ENTRIES: usize =
    WRITER_LOCK_ENTRY_COUNT + INTAKE_ENTRY_COUNT + STAGING_RESIDUE_ENTRY_COUNT;

pub(super) struct OrphanShape {
    pub(super) revision: crate::paths::directory::DirectoryRevision,
    pub(super) lock_identity: Option<crate::paths::identity::FileIdentity>,
    pub(super) saw_intake: bool,
    pub(super) residue: Option<super::StagingResidue>,
}

pub(super) fn inspect(
    invocation: &crate::paths::directory::PrivateDirectory,
    artifacts: &CodexHookArtifacts,
    staging_name: &std::ffi::CStr,
    isolation_name: &std::ffi::CStr,
) -> Result<Option<OrphanShape>> {
    let revision = invocation.revision().context(ORPHAN_CORRUPT)?;
    let lock_name = file_name(&artifacts.retirement_writer_lock_path())?;
    let intake_name = file_name(&artifacts.intake_directory())?;
    let staging_name = OsStr::from_bytes(staging_name.to_bytes());
    let isolation_name = OsStr::from_bytes(isolation_name.to_bytes());
    let mut stream = crate::paths::directory_stream::DirectoryStream::open(invocation.descriptor())
        .context(ORPHAN_CORRUPT)?;
    let mut saw_lock = false;
    let mut saw_intake = false;
    let mut residue = None;
    let mut entries = usize::default();
    while let Some(name) = stream.next_name().context(ORPHAN_CORRUPT)? {
        entries = entries
            .checked_add(ENTRY_INCREMENT)
            .context(ORPHAN_CORRUPT)?;
        ensure!(entries <= MAX_EXPECTED_ENTRIES, ORPHAN_CORRUPT);
        if name == lock_name {
            ensure!(!saw_lock, ORPHAN_CORRUPT);
            saw_lock = true;
        } else if name == intake_name {
            ensure!(!saw_intake, ORPHAN_CORRUPT);
            saw_intake = true;
        } else if name == staging_name {
            ensure!(residue.is_none(), ORPHAN_CORRUPT);
            residue = Some(super::StagingResidue::Active);
        } else if name == isolation_name {
            ensure!(residue.is_none(), ORPHAN_CORRUPT);
            residue = Some(super::StagingResidue::Isolated);
        } else {
            return Ok(None);
        }
    }
    drop(stream);
    ensure!(!saw_intake || saw_lock, ORPHAN_CORRUPT);
    ensure!(residue.is_none() || saw_intake, ORPHAN_CORRUPT);
    let lock_identity = saw_lock
        .then(|| validate_lock(invocation, artifacts))
        .transpose()?;
    if saw_intake {
        validate_empty_intake(invocation, artifacts)?;
    }
    invocation.verify_path().context(ORPHAN_CORRUPT)?;
    ensure!(invocation.revision()? == revision, ORPHAN_CORRUPT);
    Ok(Some(OrphanShape {
        revision,
        lock_identity,
        saw_intake,
        residue,
    }))
}

fn validate_lock(
    invocation: &crate::paths::directory::PrivateDirectory,
    artifacts: &CodexHookArtifacts,
) -> Result<crate::paths::identity::FileIdentity> {
    crate::paths::private_file::PrivateFileEntry::from_directory(
        invocation.try_clone()?,
        &artifacts.retirement_writer_lock_path(),
    )?
    .inspect(super::super::constants::LOCK_RECORD_MAX_BYTES)?
    .context(ORPHAN_CORRUPT)
}

fn validate_empty_intake(
    invocation: &crate::paths::directory::PrivateDirectory,
    artifacts: &CodexHookArtifacts,
) -> Result<()> {
    let intake = invocation.open_descendant(&artifacts.intake_directory())?;
    let revision = intake.revision()?;
    let mut stream = crate::paths::directory_stream::DirectoryStream::open(intake.descriptor())?;
    ensure!(stream.next_name()?.is_none(), ORPHAN_CORRUPT);
    drop(stream);
    intake.verify_path()?;
    ensure!(intake.revision()? == revision, ORPHAN_CORRUPT);
    Ok(())
}

fn file_name(path: &std::path::Path) -> Result<OsString> {
    path.file_name()
        .map(OsStr::to_owned)
        .context(ORPHAN_CORRUPT)
}
