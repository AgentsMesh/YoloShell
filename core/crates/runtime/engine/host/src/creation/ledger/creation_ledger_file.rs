use super::*;

#[cfg(test)]
pub(crate) fn read_creation_ledger<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<Option<T>> {
    read_private_durable_json(path, CREATION_LEDGER_LABEL, MAX_CREATION_LEDGER_BYTES)
}

pub(crate) fn read_creation_ledger_entry<T: serde::de::DeserializeOwned>(
    entry: &crate::paths::private_file::PrivateFileEntry,
) -> Result<Option<T>> {
    read_private_durable_json_entry_with(
        entry,
        CREATION_LEDGER_LABEL,
        MAX_CREATION_LEDGER_BYTES,
        |_| Ok(()),
    )
}

pub(in crate::host) fn read_creation_evidence<T: serde::de::DeserializeOwned>(
    path: &Path,
    label: &str,
) -> Result<Option<T>> {
    match read_private_durable_json(path, label, MAX_CREATION_EVIDENCE_BYTES) {
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
        {
            Ok(None)
        }
        result => result,
    }
}

pub(crate) fn read_private_durable_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<Option<T>> {
    read_private_durable_json_with(path, label, max_bytes, |_| Ok(()))
}

pub(crate) fn read_private_durable_json_with<T, F>(
    path: &Path,
    label: &str,
    max_bytes: u64,
    hook: F,
) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
    F: FnMut(CreationDurableReadStage) -> Result<()>,
{
    let entry = crate::paths::private_file::PrivateFileEntry::open(path)?;
    read_private_durable_json_entry_with(&entry, label, max_bytes, hook)
}

pub(crate) fn read_private_durable_json_entry_with<T, F>(
    entry: &crate::paths::private_file::PrivateFileEntry,
    label: &str,
    max_bytes: u64,
    mut hook: F,
) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
    F: FnMut(CreationDurableReadStage) -> Result<()>,
{
    use std::io::Read;
    let path = entry.path();
    if entry.inspect(max_bytes)?.is_none() {
        hook(CreationDurableReadStage::MissingObserved)?;
        if entry.inspect(max_bytes)?.is_none() {
            return Ok(None);
        }
    }
    let mut opened = entry.open_read(max_bytes)?;
    let file = opened.file_mut();
    crate::private_acl::require_empty(file, path)?;
    let identity = PrivateFileIdentity::inspect(&file.metadata()?, path, label, max_bytes)?;
    let mut bytes = Vec::new();
    Read::by_ref(file)
        .take(max_bytes + SIZE_LIMIT_SENTINEL_BYTES)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {label} {}", path.display()))?;
    anyhow::ensure!(
        bytes.len() as u64 <= max_bytes,
        "{label} exceeds byte limit: {}",
        path.display()
    );
    hook(CreationDurableReadStage::AfterRead)?;
    crate::private_acl::require_empty(file, path)?;
    identity.verify(&file.metadata()?, path, label)?;
    let (current, _) = entry.open_read(max_bytes)?.into_parts();
    crate::private_acl::require_empty(&current, path)?;
    let current_identity =
        PrivateFileIdentity::inspect(&current.metadata()?, path, label, max_bytes)?;
    anyhow::ensure!(
        identity == current_identity,
        "published {label} identity changed: {}",
        path.display()
    );
    entry.verify_current_unchanged(opened.identity(), max_bytes)?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {label} {}", path.display()))
        .map(Some)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CreationDurableReadStage {
    MissingObserved,
    AfterRead,
}

#[cfg(test)]
pub(crate) fn read_creation_ledger_with_test_hook<F>(
    path: &Path,
    hook: F,
) -> Result<Option<CreationLedger>>
where
    F: FnMut(CreationDurableReadStage) -> Result<()>,
{
    read_private_durable_json_with(path, CREATION_LEDGER_LABEL, MAX_CREATION_LEDGER_BYTES, hook)
}
