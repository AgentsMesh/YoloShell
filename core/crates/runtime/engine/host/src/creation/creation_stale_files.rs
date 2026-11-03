use super::*;

#[cfg(test)]
pub(crate) fn cleanup_stale_creation_ledgers(path: &Path) -> Result<()> {
    cleanup_stale_creation_ledgers_bounded(path, MAX_CREATION_LEDGER_PARENT_ENTRIES)
}

pub(crate) fn cleanup_stale_creation_ledger_entry(
    entry: &crate::paths::private_file::PrivateFileEntry,
) -> Result<()> {
    cleanup_stale_creation_ledger_entry_bounded(entry, MAX_CREATION_LEDGER_PARENT_ENTRIES)
}

pub(crate) fn cleanup_stale_creation_ledger_entry_bounded(
    entry: &crate::paths::private_file::PrivateFileEntry,
    max_entries: usize,
) -> Result<()> {
    let prefix = creation_ledger_temporary_prefix();
    let _lock =
        entry.lock_and_cleanup_bounded(&prefix, CREATION_LEDGER_TEMP_SUFFIX, max_entries)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn cleanup_stale_creation_ledgers_bounded(
    path: &Path,
    max_entries: usize,
) -> Result<()> {
    use crate::paths::private_file::PrivateFileEntry;
    let entry = PrivateFileEntry::open(path)?;
    let prefix = creation_ledger_temporary_prefix();
    let _lock =
        entry.lock_and_cleanup_bounded(&prefix, CREATION_LEDGER_TEMP_SUFFIX, max_entries)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn cleanup_stale_creation_ledgers_bounded_with<F>(
    path: &Path,
    max_entries: usize,
    hook: F,
) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    use crate::paths::private_file::PrivateFileEntry;
    let entry = PrivateFileEntry::open(path)?;
    let prefix = creation_ledger_temporary_prefix();
    let _lock = crate::paths::test_support::cleanup_private_files_with_hook(
        &entry,
        &prefix,
        CREATION_LEDGER_TEMP_SUFFIX,
        max_entries,
        hook,
    )?;
    Ok(())
}
