use super::catalog_durable_dir::DurableDirectory;
use super::catalog_durable_identity::DurableIdentity;
use super::constants::DURABLE_STATE_CHANGED_CONTEXT;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(crate) const MAX_DURABLE_JSON_BYTES: u64 = 1_048_576;
const SIZE_LIMIT_SENTINEL_BYTES: u64 = 1;

pub(crate) fn read_optional_durable_json<T: DeserializeOwned>(
    directory: &DurableDirectory,
    name: &OsStr,
    label: &str,
) -> Result<Option<T>> {
    read_with_hooks(directory, name, label, |_| Ok(()), |_| Ok(()))
}

fn read_with_hooks<T, F, G>(
    directory: &DurableDirectory,
    name: &OsStr,
    label: &str,
    after_metadata: F,
    after_read: G,
) -> Result<Option<T>>
where
    T: DeserializeOwned,
    F: FnOnce(&Path) -> Result<()>,
    G: FnOnce(&Path) -> Result<()>,
{
    read_with_final_hook(directory, name, label, after_metadata, after_read, |_| {
        Ok(())
    })
}

fn read_with_final_hook<T, F, G, H>(
    directory: &DurableDirectory,
    name: &OsStr,
    label: &str,
    after_metadata: F,
    after_read: G,
    after_published_version: H,
) -> Result<Option<T>>
where
    T: DeserializeOwned,
    F: FnOnce(&Path) -> Result<()>,
    G: FnOnce(&Path) -> Result<()>,
    H: FnOnce(&Path) -> Result<()>,
{
    let path = directory.entry_path(name);
    let Some(mut file) = directory
        .open_file(name)
        .with_context(|| format!("open durable {label} {}", path.display()))?
    else {
        return Ok(None);
    };
    crate::private_acl::require_empty(&file, &path)?;
    let identity = DurableIdentity::inspect(&file.metadata()?, &path)?;
    after_metadata(&path)?;
    let bytes = bounded_read(&mut file, &path, label)?;
    after_read(&path)?;
    let bytes_read = u64::try_from(bytes.len()).context("durable read length is out of range")?;
    file.seek(SeekFrom::Start(0))
        .with_context(|| format!("rewind durable {label} {}", path.display()))?;
    let verified_bytes = bounded_read(&mut file, &path, label)?;
    anyhow::ensure!(
        bytes == verified_bytes,
        "{DURABLE_STATE_CHANGED_CONTEXT}: {}",
        path.display()
    );
    identity.verify(&file.metadata()?, bytes_read, &path)?;
    #[cfg(test)]
    run_durable_read_hook(DurableReadStage::AfterDescriptorVerification, &path)?;
    let mut current = directory
        .open_file(name)?
        .with_context(|| format!("published durable {label} disappeared: {}", path.display()))?;
    crate::private_acl::require_empty(&current, &path)?;
    let current_identity = DurableIdentity::inspect(&current.metadata()?, &path)?;
    anyhow::ensure!(
        identity.same_version(&current_identity),
        "durable state version changed: {}",
        path.display()
    );
    after_published_version(&path)?;
    let published_bytes = bounded_read(&mut current, &path, label)?;
    anyhow::ensure!(
        bytes == published_bytes,
        "{DURABLE_STATE_CHANGED_CONTEXT}: {}",
        path.display()
    );
    current_identity.verify(&current.metadata()?, bytes_read, &path)?;
    identity.verify(&file.metadata()?, bytes_read, &path)?;
    #[cfg(test)]
    run_durable_read_hook(DurableReadStage::AfterPublishedVerification, &path)?;
    let mut published = directory
        .open_file(name)?
        .with_context(|| format!("published durable {label} disappeared: {}", path.display()))?;
    crate::private_acl::require_empty(&published, &path)?;
    let published_identity = DurableIdentity::inspect(&published.metadata()?, &path)?;
    anyhow::ensure!(
        current_identity.same_version(&published_identity),
        "durable state path changed: {}",
        path.display()
    );
    #[cfg(test)]
    run_durable_read_hook(DurableReadStage::BeforeFinalRead, &path)?;
    let final_bytes = bounded_read(&mut published, &path, label)?;
    anyhow::ensure!(
        published_bytes == final_bytes,
        "{DURABLE_STATE_CHANGED_CONTEXT}: {}",
        path.display()
    );
    published_identity.verify(&published.metadata()?, bytes_read, &path)?;
    directory.verify_path()?;
    identity.verify(&file.metadata()?, bytes_read, &path)?;
    current_identity.verify(&current.metadata()?, bytes_read, &path)?;
    serde_json::from_slice(&final_bytes)
        .with_context(|| format!("parse durable {label} {}", path.display()))
        .map(Some)
}

fn bounded_read(file: &mut File, path: &Path, label: &str) -> Result<Vec<u8>> {
    let read_limit = MAX_DURABLE_JSON_BYTES + SIZE_LIMIT_SENTINEL_BYTES;
    let mut bytes = Vec::new();
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read durable {label} {}", path.display()))?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_DURABLE_JSON_BYTES,
        "durable {label} exceeds byte limit: {}",
        path.display()
    );
    Ok(bytes)
}

#[cfg(test)]
pub(crate) fn read_manifest_with_hooks<F, G>(
    directory: &DurableDirectory,
    name: &OsStr,
    after_metadata: F,
    after_read: G,
) -> Result<Option<crate::model::TerminalManifest>>
where
    F: FnOnce(&Path) -> Result<()>,
    G: FnOnce(&Path) -> Result<()>,
{
    read_with_hooks(
        directory,
        name,
        "session manifest",
        after_metadata,
        after_read,
    )
}

#[cfg(test)]
pub(crate) fn read_manifest_with_final_hook<F>(
    directory: &DurableDirectory,
    name: &OsStr,
    after_published_version: F,
) -> Result<Option<crate::model::TerminalManifest>>
where
    F: FnOnce(&Path) -> Result<()>,
{
    read_with_final_hook(
        directory,
        name,
        "session manifest",
        |_| Ok(()),
        |_| Ok(()),
        after_published_version,
    )
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../test_support/unit/catalog_durable_file_test_support.rs");
