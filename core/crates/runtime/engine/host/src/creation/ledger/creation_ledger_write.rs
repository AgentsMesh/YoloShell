use super::*;

pub(crate) enum CreationLedgerWriteFailure {
    Unchanged(anyhow::Error),
    Uncertain(anyhow::Error),
}

impl CreationLedgerWriteFailure {
    pub(crate) fn into_error(self) -> anyhow::Error {
        match self {
            Self::Unchanged(error) | Self::Uncertain(error) => error,
        }
    }

    pub(crate) fn is_publication_deadline_expired(&self) -> bool {
        match self {
            Self::Unchanged(error) | Self::Uncertain(error) => {
                error.is::<CreationPublicationDeadlineExpired>()
            }
        }
    }
}

pub(crate) fn write_creation_ledger<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = encode_creation_ledger(value, path)?;
    write_encoded_creation_ledger(path, &encoded).map_err(CreationLedgerWriteFailure::into_error)
}

pub(crate) fn write_creation_ledger_entry<T: Serialize>(
    entry: &crate::paths::private_file::PrivateFileEntry,
    value: &T,
) -> Result<()> {
    let encoded = encode_creation_ledger(value, entry.path())?;
    write_encoded_creation_ledger_entry(entry, &encoded)
        .map_err(CreationLedgerWriteFailure::into_error)
}

pub(crate) fn encode_creation_ledger<T: Serialize>(value: &T, path: &Path) -> Result<Vec<u8>> {
    let encoded = serde_json::to_vec_pretty(value)?;
    anyhow::ensure!(
        encoded.len() as u64 <= MAX_CREATION_LEDGER_BYTES,
        "creation ledger exceeds byte limit: {}",
        path.display()
    );
    Ok(encoded)
}

pub(crate) fn write_encoded_creation_ledger(
    path: &Path,
    encoded: &[u8],
) -> std::result::Result<(), CreationLedgerWriteFailure> {
    write_encoded_creation_ledger_with(path, encoded, |_| Ok(()))
}

pub(crate) fn write_encoded_creation_ledger_entry(
    entry: &crate::paths::private_file::PrivateFileEntry,
    encoded: &[u8],
) -> std::result::Result<(), CreationLedgerWriteFailure> {
    write_encoded_creation_ledger_entry_with(entry, encoded, |_| Ok(()))
}

pub(crate) fn write_encoded_creation_ledger_with<F>(
    path: &Path,
    encoded: &[u8],
    hook: F,
) -> std::result::Result<(), CreationLedgerWriteFailure>
where
    F: FnMut(CreationLedgerWriteStage) -> Result<()>,
{
    use crate::paths::private_file::PrivateFileEntry;
    use CreationLedgerWriteFailure::Unchanged;
    let entry = PrivateFileEntry::ensure(path).map_err(Unchanged)?;
    write_encoded_creation_ledger_entry_with(&entry, encoded, hook)
}

pub(crate) fn write_encoded_creation_ledger_entry_with<F>(
    entry: &crate::paths::private_file::PrivateFileEntry,
    encoded: &[u8],
    hook: F,
) -> std::result::Result<(), CreationLedgerWriteFailure>
where
    F: FnMut(CreationLedgerWriteStage) -> Result<()>,
{
    write_encoded_creation_ledger_entry_with_fence(entry, encoded, hook, || Ok(()))
}

pub(crate) fn write_encoded_creation_ledger_entry_with_fence<F, G>(
    entry: &crate::paths::private_file::PrivateFileEntry,
    encoded: &[u8],
    mut hook: F,
    mut commit_fence: G,
) -> std::result::Result<(), CreationLedgerWriteFailure>
where
    F: FnMut(CreationLedgerWriteStage) -> Result<()>,
    G: FnMut() -> Result<()>,
{
    use CreationLedgerWriteFailure::{Uncertain, Unchanged};
    let publication_is_uncertain = std::cell::Cell::new(false);
    hook(CreationLedgerWriteStage::ParentOpened).map_err(Unchanged)?;
    let prefix = creation_ledger_temporary_prefix();
    let mut temporary = entry
        .temporary(&prefix, CREATION_LEDGER_TEMP_SUFFIX)
        .map_err(Unchanged)?;
    let result = (|| -> Result<()> {
        seal_creation_temporary(temporary.file_mut(), entry.path(), encoded)?;
        temporary.sync_and_inspect(entry.path(), MAX_CREATION_LEDGER_BYTES)?;
        entry.rename_temporary_with_fences(
            &temporary,
            MAX_CREATION_LEDGER_BYTES,
            || match hook(CreationLedgerWriteStage::BeforePublish) {
                Ok(()) => {
                    publication_is_uncertain.set(true);
                    Ok(())
                }
                Err(error) => {
                    publication_is_uncertain.set(!error.is::<CreationPublicationDeadlineExpired>());
                    Err(error)
                }
            },
            || match commit_fence() {
                Ok(()) => {
                    publication_is_uncertain.set(true);
                    Ok(())
                }
                Err(error) => {
                    publication_is_uncertain.set(!error.is::<CreationPublicationDeadlineExpired>());
                    Err(error)
                }
            },
            || {},
        )?;
        hook(CreationLedgerWriteStage::Published)?;
        entry.verify_current_content_version(temporary.identity(), MAX_CREATION_LEDGER_BYTES)
    })();
    let result = result.map_err(|error| match entry.remove_temporary(&temporary) {
        Ok(()) => error,
        Err(cleanup) => error.context(format!(
            "creation ledger staging cleanup also failed: {cleanup:#}"
        )),
    });
    result.map_err(|error| {
        if publication_is_uncertain.get() {
            Uncertain(error)
        } else {
            Unchanged(error)
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CreationLedgerWriteStage {
    ParentOpened,
    BeforePublish,
    Published,
}

#[cfg(test)]
pub(crate) fn write_creation_ledger_with_test_hook<T, F>(
    path: &Path,
    value: &T,
    hook: F,
) -> std::result::Result<(), CreationLedgerWriteFailure>
where
    T: Serialize,
    F: FnMut(CreationLedgerWriteStage) -> Result<()>,
{
    let encoded =
        encode_creation_ledger(value, path).map_err(CreationLedgerWriteFailure::Unchanged)?;
    write_encoded_creation_ledger_with(path, &encoded, hook)
}

pub(crate) fn seal_creation_temporary(file: &mut File, path: &Path, encoded: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::os::fd::AsRawFd;
    let chmod =
        unsafe { libc::fchmod(file.as_raw_fd(), CREATION_LEDGER_FILE_MODE as libc::mode_t) };
    if chmod != 0 {
        return Err(std::io::Error::last_os_error())
            .context("set temporary creation ledger permissions");
    }
    crate::private_acl::clear(file, path)?;
    file.write_all(encoded)?;
    file.sync_all().context("sync temporary creation ledger")
}
