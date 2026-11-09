use super::constants::*;
use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;

pub fn atomic_write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    super::validation::validate_absolute(path)?;
    let encoded = encode(path, value)?;
    #[cfg(unix)]
    return write_unix(path, &encoded, |_| Ok(()));
    #[cfg(not(unix))]
    anyhow::bail!(
        "secure durable writes require Unix descriptor APIs: {}",
        path.display()
    )
}

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    super::validation::validate_absolute(path)?;
    #[cfg(unix)]
    return read_unix(path, |_| Ok(()));
    #[cfg(not(unix))]
    anyhow::bail!(
        "secure durable reads require Unix descriptor APIs: {}",
        path.display()
    )
}

pub(super) fn encode<T: Serialize>(path: &Path, value: &T) -> Result<Vec<u8>> {
    let encoded = serde_json::to_vec_pretty(value)?;
    anyhow::ensure!(
        u64::try_from(encoded.len())? <= MAX_DURABLE_JSON_BYTES,
        "state exceeds byte limit: {}",
        path.display()
    );
    Ok(encoded)
}

#[cfg(unix)]
use super::durable_json_read::{bounded_read, read_unix};
#[cfg(unix)]
use super::identity::FileIdentity;
#[cfg(unix)]
use super::private_file::{
    ParentLockAuthority, PrivateFileEntry, TargetLockAuthority, TemporaryPrivateFile,
};
#[cfg(unix)]
use std::io::Write;

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DurableReadStage {
    AfterMetadata,
    AfterFirstRead,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DurableWriteStage {
    TemporarySynced,
    Installed,
}

#[cfg(unix)]
fn write_unix<F>(path: &Path, encoded: &[u8], hook: F) -> Result<()>
where
    F: FnMut(DurableWriteStage) -> Result<()>,
{
    write_unix_with(path, encoded, hook, || Ok(()))
}

#[cfg(unix)]
fn write_unix_with<F, G>(path: &Path, encoded: &[u8], mut hook: F, mut commit: G) -> Result<()>
where
    F: FnMut(DurableWriteStage) -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let entry = PrivateFileEntry::ensure(path)?;
    write_unix_entry(&entry, encoded, &mut hook, &mut commit)
}

include!("durable_json_commit.rs");
include!("durable_json_commit_serialized.rs");
include!("durable_json_write.rs");
include!("durable_json_write_serialized.rs");

#[cfg(all(test, unix))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../../test_support/unit/durable_json_test_api.rs");
