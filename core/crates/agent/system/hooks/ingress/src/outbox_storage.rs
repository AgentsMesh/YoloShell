use super::Outbox;
use crate::constants::{INVALID, MAX_OUTBOX_BYTES, OUTBOX_LOCK_RETRY, OUTBOX_LOCK_TIMEOUT};
use anyhow::{Result, ensure};
use std::path::Path;
use yolo_paths::private_file::PrivateFileEntry;

/// One writer at a time: hook callbacks for different events can run
/// concurrently, so load-mutate-store under the durable parent lock is what
/// keeps a queue entry from being lost by an interleaved writer.
pub(super) fn update<T>(path: &Path, mutate: impl FnOnce(&mut Outbox) -> Result<T>) -> Result<T> {
    let entry = PrivateFileEntry::open(path)?;
    let _lock = entry.lock_parent_exclusive_wait(OUTBOX_LOCK_TIMEOUT, OUTBOX_LOCK_RETRY)?;
    let (mut state, identity) = load(&entry)?;
    let result = mutate(&mut state)?;
    state.validate()?;
    let bytes = serde_json::to_vec(&state)?;
    ensure!(bytes.len() as u64 <= MAX_OUTBOX_BYTES, INVALID);
    let verifier = entry.reopen()?;
    yolo_paths::write_expected_entry_guarded_with_commit_observer(
        entry,
        identity,
        &bytes,
        || Ok(()),
        || match identity {
            Some(identity) => verifier.verify_exact_identity(identity, MAX_OUTBOX_BYTES),
            None => {
                ensure!(verifier.is_missing()?, INVALID);
                Ok(())
            }
        },
        |_| {},
    )?;
    Ok(result)
}

pub(super) fn read<T>(path: &Path, inspect: impl FnOnce(&Outbox) -> Result<T>) -> Result<T> {
    let entry = PrivateFileEntry::open(path)?;
    let _lock = entry.lock_parent_exclusive_wait(OUTBOX_LOCK_TIMEOUT, OUTBOX_LOCK_RETRY)?;
    let (state, _) = load(&entry)?;
    inspect(&state)
}

fn load(
    entry: &PrivateFileEntry,
) -> Result<(Outbox, Option<yolo_paths::identity::FileIdentity>)> {
    if entry.is_missing()? {
        return Ok((Outbox::default(), None));
    }
    let (state, identity): (Outbox, _) =
        yolo_paths::read_entry_guarded_with_identity(entry.reopen()?, || Ok(()))?;
    state.validate()?;
    Ok((state, Some(identity)))
}
