use super::constants::{MAX_SNAPSHOT_BYTES, errors};
use super::snapshot_lock::SnapshotLock;
use crate::agent::hooks::ReducerSnapshot;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::cell::Cell;
use std::path::Path;

#[path = "snapshot_storage_failure.rs"]
mod failure;
#[path = "snapshot_storage_hooks.rs"]
mod hooks;
pub(super) use failure::classify_commit_failure;

pub(super) struct LoadedSnapshot {
    pub(super) snapshot: Option<ReducerSnapshot>,
    pub(super) identity: Option<FileIdentity>,
}

pub(super) fn encode(snapshot: &ReducerSnapshot) -> Result<Vec<u8>> {
    let encoded = serde_json::to_vec(snapshot).context(errors::SNAPSHOT_CORRUPT)?;
    ensure!(
        u64::try_from(encoded.len())? <= MAX_SNAPSHOT_BYTES,
        errors::SNAPSHOT_CORRUPT
    );
    Ok(encoded)
}

pub(super) fn load(lock: &SnapshotLock, path: &Path) -> Result<LoadedSnapshot> {
    let expected = lock.target_identity();
    load_expected(lock, path, expected)
}

pub(super) fn load_expected(
    lock: &SnapshotLock,
    path: &Path,
    expected: Option<FileIdentity>,
) -> Result<LoadedSnapshot> {
    match load_inner(lock, path, expected) {
        Ok(loaded) => Ok(loaded),
        Err(error) => Err(failure::classify_read_failure(lock, path, expected, error)),
    }
}

fn load_inner(
    lock: &SnapshotLock,
    path: &Path,
    expected: Option<FileIdentity>,
) -> Result<LoadedSnapshot> {
    lock.verify()?;
    let entry = lock.entry(path)?;
    let initial = entry
        .inspect(MAX_SNAPSHOT_BYTES)
        .context(errors::SNAPSHOT_CORRUPT)?;
    ensure!(initial == expected, errors::SNAPSHOT_CORRUPT);
    hooks::run_before_read();
    let loaded = match initial {
        Some(initial_identity) => {
            let (snapshot, identity) =
                crate::paths::read_entry_guarded_with_identity(entry, || {
                    lock.verify()?;
                    hooks::run_after_open();
                    lock.verify()
                })
                .map_err(|error| error.context(errors::SNAPSHOT_CORRUPT))?;
            ensure!(
                identity.unchanged(initial_identity),
                errors::SNAPSHOT_CORRUPT
            );
            LoadedSnapshot {
                snapshot: Some(snapshot),
                identity: Some(identity),
            }
        }
        None => LoadedSnapshot {
            snapshot: None,
            identity: None,
        },
    };
    lock.verify()?;
    if loaded.identity.is_none() {
        ensure!(lock.entry(path)?.is_missing()?, errors::SNAPSHOT_CORRUPT);
        lock.verify()?;
    }
    Ok(loaded)
}

pub(super) fn persist(
    lock: &SnapshotLock,
    path: &Path,
    expected: Option<FileIdentity>,
    snapshot: &ReducerSnapshot,
) -> Result<FileIdentity> {
    if lock.target_identity() != expected {
        return Err(super::snapshot_error::snapshot_changed());
    }
    lock.verify()?;
    let encoded = encode(snapshot)?;
    let committed = Cell::new(None);
    let result = crate::paths::write_expected_entry_guarded_with_commit_observer(
        lock.entry(path)?,
        expected,
        &encoded,
        || lock.verify(),
        || {
            hooks::run_before_persist_commit();
            prove_current(lock, path, expected)
        },
        |identity| committed.set(Some(identity)),
    );
    let identity = result.map_err(|error| {
        failure::classify_commit_failure(
            lock,
            path,
            expected,
            committed.get(),
            error.context(errors::SNAPSHOT_CORRUPT),
        )
    })?;
    hooks::run_after_persist();
    verify_current(lock, path, Some(identity))?;
    Ok(identity)
}

pub(super) fn prove_current(
    lock: &SnapshotLock,
    path: &Path,
    expected: Option<FileIdentity>,
) -> Result<()> {
    if lock.target_identity() != expected {
        return Err(super::snapshot_error::snapshot_changed());
    }
    verify_current(lock, path, expected)
}

pub(super) fn prove_retained_current(
    lock: &SnapshotLock,
    path: &Path,
    expected: FileIdentity,
) -> Result<()> {
    verify_current(lock, path, Some(expected))
}

fn verify_current(lock: &SnapshotLock, path: &Path, expected: Option<FileIdentity>) -> Result<()> {
    let current = failure::observe_current(lock, path)?;
    if current != expected {
        return Err(super::snapshot_error::snapshot_changed());
    }
    Ok(())
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) use hooks::{
    set_after_open_hook, set_after_persist_hook, set_before_persist_commit_hook,
    set_before_read_hook,
};
