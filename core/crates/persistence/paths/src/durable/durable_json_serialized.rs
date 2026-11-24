use super::durable_json::encode;
use super::identity::FileIdentity;
use super::private_file::ParentLockAuthority;
use super::private_file::PrivateFileEntry;
use anyhow::Result;
use serde::Serialize;

pub(crate) fn write_entry_expected_serialized<T, F, G, H>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    value: &T,
    authority: &ParentLockAuthority<'_>,
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
    super::durable_json::write_expected_entry_guarded_serialized(
        entry,
        expected,
        &encoded,
        authority,
        guard,
        commit_fence,
        after_commit,
    )
}
