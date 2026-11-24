use super::constants::{MAX_OUTBOX_BYTES, errors};
use super::lock::OutboxLock;
use super::outbox::CodexOutboxState;
use super::outbox_validation;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::cell::Cell;
use std::path::Path;

#[path = "outbox_storage_hooks.rs"]
mod hooks;

pub(super) use hooks::run_after_operation_hook;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) use hooks::set_after_operation_hook;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub use hooks::set_before_persist_commit_hook;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub use hooks::set_before_persist_reproof_hook;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) use hooks::{set_after_load_hook, set_after_open_hook, set_before_read_hook};

pub(super) struct LoadedOutbox {
    pub(super) state: CodexOutboxState,
    identity: Option<FileIdentity>,
}

pub(super) fn load(lock: &OutboxLock, path: &Path) -> Result<LoadedOutbox> {
    lock.verify()?;
    let entry = lock.entry(path)?;
    let initial = entry.inspect(u64::try_from(MAX_OUTBOX_BYTES)?)?;
    ensure!(initial == lock.target_identity(), errors::OUTBOX_CORRUPT);
    hooks::run_before_read_hook();
    let loaded = match initial {
        Some(initial_identity) => {
            let (state, identity) = crate::paths::read_entry_guarded_with_identity(entry, || {
                lock.verify()?;
                hooks::run_after_open_hook();
                lock.verify()
            })
            .map_err(|error| error.context(errors::OUTBOX_CORRUPT))?;
            ensure!(identity.unchanged(initial_identity), errors::OUTBOX_CORRUPT);
            outbox_validation::validate(&state)?;
            LoadedOutbox {
                state,
                identity: Some(identity),
            }
        }
        None => LoadedOutbox {
            state: CodexOutboxState::default(),
            identity: None,
        },
    };
    hooks::run_after_load_hook();
    lock.verify()?;
    if loaded.identity.is_none() {
        let entry = lock.entry(path)?;
        ensure!(entry.is_missing()?, errors::OUTBOX_CORRUPT);
        lock.verify()?;
    }
    Ok(loaded)
}

pub(super) fn persist(lock: &OutboxLock, path: &Path, loaded: &LoadedOutbox) -> Result<()> {
    persist_with_commit_fence(lock, path, loaded, || Ok(()))
}

pub(super) fn persist_with_commit_fence(
    lock: &OutboxLock,
    path: &Path,
    loaded: &LoadedOutbox,
    mut commit_fence: impl FnMut() -> Result<()>,
) -> Result<()> {
    let state = &loaded.state;
    outbox_validation::validate(state)?;
    let encoded = serde_json::to_vec(state).context(errors::OUTBOX_CORRUPT)?;
    ensure!(encoded.len() <= MAX_OUTBOX_BYTES, errors::OUTBOX_FULL);
    lock.verify()?;
    let entry = lock.entry(path)?;
    let committed = Cell::new(None);
    let result = crate::paths::write_expected_entry_guarded_with_commit_observer(
        entry,
        loaded.identity,
        &encoded,
        || lock.verify(),
        || {
            hooks::run_before_persist_commit_hook(path);
            lock.verify()?;
            commit_fence()?;
            lock.verify()
        },
        |identity| committed.set(Some(identity)),
    );
    match result {
        Ok(_) => lock.verify().context(errors::OUTBOX_CORRUPT),
        Err(error) => recover_post_commit_failure(lock, path, state, committed.get(), error),
    }
}

fn recover_post_commit_failure(
    lock: &OutboxLock,
    path: &Path,
    expected: &CodexOutboxState,
    committed: Option<FileIdentity>,
    error: anyhow::Error,
) -> Result<()> {
    let Some(committed) = committed else {
        return Err(error.context(errors::OUTBOX_CORRUPT));
    };
    hooks::run_before_persist_reproof_hook(path);
    let reproof = verify_committed(lock, path, expected, committed).and_then(|()| {
        lock.verify()?;
        lock.entry(path)?.sync_parent()?;
        verify_committed(lock, path, expected, committed)
    });
    match reproof {
        Ok(()) => Ok(()),
        Err(reproof) => Err(super::outbox_storage_error::commit_indeterminate(
            error.context(errors::OUTBOX_CORRUPT),
            reproof.context(errors::OUTBOX_COMMIT_REPROOF_FAILED),
        )),
    }
}

fn verify_committed(
    lock: &OutboxLock,
    path: &Path,
    expected: &CodexOutboxState,
    committed: FileIdentity,
) -> Result<()> {
    let max_bytes = u64::try_from(MAX_OUTBOX_BYTES)?;
    lock.verify()?;
    let entry = lock.entry(path)?;
    let inspected = entry
        .inspect(max_bytes)?
        .context(errors::OUTBOX_COMMIT_REPROOF_FAILED)?;
    ensure!(
        inspected.same_content_version(committed),
        errors::OUTBOX_COMMIT_REPROOF_FAILED
    );
    let (current, identity) =
        crate::paths::read_entry_guarded_with_identity(entry, || lock.verify())?;
    ensure!(
        identity.unchanged(inspected) && identity.same_content_version(committed),
        errors::OUTBOX_COMMIT_REPROOF_FAILED
    );
    ensure!(current == *expected, errors::OUTBOX_COMMIT_REPROOF_FAILED);
    outbox_validation::validate(&current)?;
    lock.verify()?;
    let current = lock.entry(path)?.inspect(max_bytes)?;
    ensure!(
        current == Some(identity),
        errors::OUTBOX_COMMIT_REPROOF_FAILED
    );
    lock.verify()
}

pub(super) fn verify_unchanged(
    lock: &OutboxLock,
    path: &Path,
    loaded: &LoadedOutbox,
) -> Result<()> {
    lock.verify()?;
    let current = lock
        .entry(path)?
        .inspect(u64::try_from(MAX_OUTBOX_BYTES)?)?;
    ensure!(current == loaded.identity, errors::OUTBOX_CORRUPT);
    lock.verify().context(errors::OUTBOX_CORRUPT)
}
