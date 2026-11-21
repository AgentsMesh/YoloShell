use super::durable_json::{encode, write_entry_guarded};
use super::durable_json_read::read_entry_guarded as read_entry;
use super::durable_json_read::read_entry_guarded_with_identity as read_entry_with_identity;
use super::identity::FileIdentity;
use super::private_file::{PrivateFileEntry, TargetLockAuthority};
use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn atomic_write_entry_guarded<T, F>(entry: PrivateFileEntry, value: &T, guard: F) -> Result<()>
where
    T: Serialize,
    F: FnOnce() -> Result<()>,
{
    let encoded = encode(entry.path(), value)?;
    write_entry_guarded(entry, &encoded, guard)
}

pub(crate) fn atomic_write_entry_guarded_with_commit_fence<T, F, G>(
    entry: PrivateFileEntry,
    value: &T,
    guard: F,
    commit_fence: G,
) -> Result<()>
where
    T: Serialize,
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let encoded = encode(entry.path(), value)?;
    super::durable_json::write_entry_guarded_with_commit_fence(entry, &encoded, guard, commit_fence)
}

pub(crate) fn write_entry_expected_with_commit_fence<T, F, G>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    value: &T,
    guard: F,
    commit_fence: G,
) -> Result<FileIdentity>
where
    T: Serialize,
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let encoded = encode(entry.path(), value)?;
    super::durable_json::write_expected_entry_guarded_with_commit_fence(
        entry,
        expected,
        &encoded,
        guard,
        commit_fence,
    )
}

pub(crate) fn write_entry_expected_with_commit_observer<T, F, G, H>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    value: &T,
    guard: F,
    commit_fence: G,
    after_commit: H,
) -> Result<FileIdentity>
where
    T: Serialize,
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut(FileIdentity),
{
    let encoded = encode(entry.path(), value)?;
    super::durable_json::write_expected_entry_guarded_with_commit_observer(
        entry,
        expected,
        &encoded,
        guard,
        commit_fence,
        after_commit,
    )
}

pub(crate) fn write_entry_expected_with_commit_fence_with_target_lock_authority<T, F, G>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    value: &T,
    authority: &TargetLockAuthority<'_>,
    guard: F,
    commit_fence: G,
) -> Result<FileIdentity>
where
    T: Serialize,
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let encoded = encode(entry.path(), value)?;
    super::durable_json::write_expected_entry_guarded_with_commit_fence_with_target_lock_authority(
        entry,
        expected,
        &encoded,
        authority,
        guard,
        commit_fence,
    )
}

pub(crate) fn write_entry_expected_with_commit_observer_with_target_lock_authority<T, F, G, H>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    value: &T,
    authority: &TargetLockAuthority<'_>,
    guard: F,
    commit_fence: G,
    after_commit: H,
) -> Result<FileIdentity>
where
    T: Serialize,
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut(FileIdentity),
{
    let encoded = encode(entry.path(), value)?;
    super::durable_json::write_expected_entry_guarded_with_commit_observer_with_target_lock_authority(
        entry,
        expected,
        &encoded,
        authority,
        guard,
        commit_fence,
        after_commit,
    )
}

pub(super) fn replace_entry_guarded<T, F>(
    entry: PrivateFileEntry,
    expected: FileIdentity,
    value: &T,
    guard: F,
) -> Result<()>
where
    T: Serialize,
    F: FnOnce() -> Result<()>,
{
    let encoded = encode(entry.path(), value)?;
    super::durable_json::replace_expected_entry(entry, expected, &encoded, guard)
}

pub fn read_entry_guarded<T, F>(entry: PrivateFileEntry, guard: F) -> Result<T>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    read_entry(entry, guard)
}

pub fn read_entry_guarded_with_identity<T, F>(
    entry: PrivateFileEntry,
    guard: F,
) -> Result<(T, FileIdentity)>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    read_entry_with_identity(entry, guard)
}

pub fn read_entry_guarded_with_versions<T, F>(
    entry: PrivateFileEntry,
    guard: F,
) -> Result<(T, FileIdentity, super::identity::DurableFileVersion)>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    super::durable_json_read::read_entry_guarded_with_versions(entry, guard)
}
