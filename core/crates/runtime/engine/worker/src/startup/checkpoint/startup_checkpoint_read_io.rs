use super::StartupCheckpointEvidence;
use super::constants::*;
use crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::{OpenedPrivateFile, PrivateFileEntry};
use anyhow::{Context, Result};
use std::io::{Read, Seek, SeekFrom};

pub(crate) fn read_opened<F>(
    entry: PrivateFileEntry,
    mut opened: OpenedPrivateFile,
    guard: F,
) -> Result<(StartupCheckpointEvidence, FileIdentity)>
where
    F: FnOnce() -> Result<()>,
{
    let path = entry.path().to_owned();
    guard()?;
    let first = read_bytes(opened.file_mut())?;
    verify_read(&entry, &opened, &first)?;
    opened.file_mut().seek(SeekFrom::Start(0))?;
    let second = read_bytes(opened.file_mut())?;
    anyhow::ensure!(first == second, CHANGED_ERROR);
    verify_read(&entry, &opened, &second)?;
    let evidence = serde_json::from_slice(&first)
        .with_context(|| format!("{PARSE_CONTEXT}: {}", path.display()))?;
    Ok((evidence, opened.identity()))
}

pub(crate) fn read_bytes(file: &mut std::fs::File) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(WORKER_STARTUP_CHECKPOINT_MAX_BYTES + SIZE_SENTINEL_BYTES)
        .read_to_end(&mut bytes)
        .context(READ_CONTEXT)?;
    anyhow::ensure!(
        u64::try_from(bytes.len())? <= WORKER_STARTUP_CHECKPOINT_MAX_BYTES,
        READ_CONTEXT
    );
    Ok(bytes)
}

pub(crate) fn verify_read(
    entry: &PrivateFileEntry,
    opened: &OpenedPrivateFile,
    bytes: &[u8],
) -> Result<()> {
    opened.identity().verify_file_bounded(
        opened.file(),
        u64::try_from(bytes.len())?,
        entry.path(),
        WORKER_STARTUP_CHECKPOINT_MAX_BYTES,
    )?;
    entry.verify_current_unchanged(opened.identity(), WORKER_STARTUP_CHECKPOINT_MAX_BYTES)
}
