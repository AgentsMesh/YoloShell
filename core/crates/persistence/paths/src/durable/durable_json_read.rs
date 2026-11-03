use super::constants::{MAX_DURABLE_JSON_BYTES, SIZE_LIMIT_SENTINEL_BYTES};
use super::durable_json::DurableReadStage;
use super::identity::{DurableFileVersion, FileIdentity};
use super::private_file::{OpenedPrivateFile, PrivateFileEntry};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const PARSE_STATE_DETAIL: &str = "parse state";

pub(super) fn read_unix<T, F>(path: &Path, mut hook: F) -> Result<T>
where
    T: DeserializeOwned,
    F: FnMut(DurableReadStage) -> Result<()>,
{
    read_unix_with(path, &mut hook, || Ok(()))
}

pub(super) fn read_entry_guarded<T, F>(entry: PrivateFileEntry, guard: F) -> Result<T>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    read_entry_with(&entry, &mut |_| Ok(()), guard)
}

pub(super) fn read_entry_guarded_with_identity<T, F>(
    entry: PrivateFileEntry,
    guard: F,
) -> Result<(T, FileIdentity)>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    read_entry_with_identity(&entry, &mut |_| Ok(()), guard)
}

pub(super) fn read_retained_entry_guarded_with_opened<T, F>(
    entry: &PrivateFileEntry,
    guard: F,
) -> Result<(T, OpenedPrivateFile)>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    let path = entry.path();
    let mut opened = entry.open_retained(MAX_DURABLE_JSON_BYTES)?;
    guard()?;
    let first = bounded_read(opened.file_mut(), path)?;
    opened.identity().verify_file_bounded(
        opened.file(),
        u64::try_from(first.len())?,
        path,
        MAX_DURABLE_JSON_BYTES,
    )?;
    opened.file_mut().seek(SeekFrom::Start(0))?;
    let second = bounded_read(opened.file_mut(), path)?;
    anyhow::ensure!(first == second, crate::ReadVersionChanged::state(path));
    opened.identity().verify_file_bounded(
        opened.file(),
        u64::try_from(second.len())?,
        path,
        MAX_DURABLE_JSON_BYTES,
    )?;
    entry.verify_retained_path_unchanged(opened.identity(), MAX_DURABLE_JSON_BYTES)?;
    let value = serde_json::from_slice(&first)
        .with_context(|| format!("{PARSE_STATE_DETAIL} {}", path.display()))?;
    Ok((value, opened))
}

pub(super) fn read_entry_guarded_with_versions<T, F>(
    entry: PrivateFileEntry,
    guard: F,
) -> Result<(T, FileIdentity, DurableFileVersion)>
where
    T: DeserializeOwned,
    F: FnOnce() -> Result<()>,
{
    let (value, opened) = read_entry_with_opened(&entry, &mut |_| Ok(()), guard)?;
    let durable = opened.durable_version(entry.path(), MAX_DURABLE_JSON_BYTES)?;
    let live = opened.identity();
    entry.verify_current_unchanged(live, MAX_DURABLE_JSON_BYTES)?;
    Ok((value, live, durable))
}

fn read_unix_with<T, F, G>(path: &Path, hook: &mut F, guard: G) -> Result<T>
where
    T: DeserializeOwned,
    F: FnMut(DurableReadStage) -> Result<()>,
    G: FnOnce() -> Result<()>,
{
    let entry = PrivateFileEntry::open(path)?;
    read_entry_with(&entry, hook, guard)
}

fn read_entry_with<T, F, G>(entry: &PrivateFileEntry, hook: &mut F, guard: G) -> Result<T>
where
    T: DeserializeOwned,
    F: FnMut(DurableReadStage) -> Result<()>,
    G: FnOnce() -> Result<()>,
{
    read_entry_with_identity(entry, hook, guard).map(|(value, _)| value)
}

fn read_entry_with_identity<T, F, G>(
    entry: &PrivateFileEntry,
    hook: &mut F,
    guard: G,
) -> Result<(T, FileIdentity)>
where
    T: DeserializeOwned,
    F: FnMut(DurableReadStage) -> Result<()>,
    G: FnOnce() -> Result<()>,
{
    let (value, opened) = read_entry_with_opened(entry, hook, guard)?;
    Ok((value, opened.identity()))
}

fn read_entry_with_opened<T, F, G>(
    entry: &PrivateFileEntry,
    hook: &mut F,
    guard: G,
) -> Result<(T, OpenedPrivateFile)>
where
    T: DeserializeOwned,
    F: FnMut(DurableReadStage) -> Result<()>,
    G: FnOnce() -> Result<()>,
{
    let path = entry.path();
    let mut opened = entry.open_read(MAX_DURABLE_JSON_BYTES)?;
    guard()?;
    hook(DurableReadStage::AfterMetadata)?;
    let first = bounded_read(&mut opened.file, path)?;
    opened.identity.verify_file_bounded(
        &opened.file,
        u64::try_from(first.len())?,
        path,
        MAX_DURABLE_JSON_BYTES,
    )?;
    hook(DurableReadStage::AfterFirstRead)?;
    opened.file.seek(SeekFrom::Start(0))?;
    let second = bounded_read(&mut opened.file, path)?;
    anyhow::ensure!(first == second, crate::ReadVersionChanged::state(path));
    opened.identity.verify_file_bounded(
        &opened.file,
        u64::try_from(second.len())?,
        path,
        MAX_DURABLE_JSON_BYTES,
    )?;
    entry.verify_current_unchanged(opened.identity, MAX_DURABLE_JSON_BYTES)?;
    let value = serde_json::from_slice(&first)
        .with_context(|| format!("{PARSE_STATE_DETAIL} {}", path.display()))?;
    Ok((value, opened))
}

pub(super) fn bounded_read(file: &mut std::fs::File, path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(MAX_DURABLE_JSON_BYTES + SIZE_LIMIT_SENTINEL_BYTES)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read state {}", path.display()))?;
    anyhow::ensure!(
        u64::try_from(bytes.len())? <= MAX_DURABLE_JSON_BYTES,
        "state exceeds byte limit: {}",
        path.display()
    );
    Ok(bytes)
}
