use super::private_file::{OpenedPrivateFile, PrivateFileEntry};
use anyhow::Result;
use serde::de::DeserializeOwned;

pub(crate) fn read_retained_entry_guarded_with_opened<T, F>(
    entry: &PrivateFileEntry,
    guard: F,
) -> Result<(T, OpenedPrivateFile)>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    super::durable_json_read::read_retained_entry_guarded_with_opened(entry, guard)
}
