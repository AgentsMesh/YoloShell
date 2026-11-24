use super::{INVALID, Outbox};
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Result, ensure};
use std::path::Path;

pub(super) fn update<T>(path: &Path, mutate: impl FnOnce(&mut Outbox) -> Result<T>) -> Result<T> {
    let entry = PrivateFileEntry::open(path)?;
    let _lock = entry.lock_parent_exclusive_wait(
        crate::CLAUDE_OUTBOX_LOCK_TIMEOUT,
        crate::CLAUDE_OUTBOX_LOCK_RETRY,
    )?;
    let (mut state, identity) = load(&entry)?;
    let result = mutate(&mut state)?;
    state.validate()?;
    let bytes = serde_json::to_vec(&state)?;
    ensure!(
        bytes.len() as u64 <= crate::MAX_CLAUDE_OUTBOX_BYTES,
        INVALID
    );
    let verifier = entry.reopen()?;
    crate::paths::write_expected_entry_guarded_with_commit_observer(
        entry,
        identity,
        &bytes,
        || Ok(()),
        || match identity {
            Some(identity) => {
                verifier.verify_exact_identity(identity, crate::MAX_CLAUDE_OUTBOX_BYTES)
            }
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
    let _lock = entry.lock_parent_exclusive_wait(
        crate::CLAUDE_OUTBOX_LOCK_TIMEOUT,
        crate::CLAUDE_OUTBOX_LOCK_RETRY,
    )?;
    let (state, _) = load(&entry)?;
    inspect(&state)
}

fn load(
    entry: &PrivateFileEntry,
) -> Result<(Outbox, Option<crate::paths::identity::FileIdentity>)> {
    if entry.is_missing()? {
        return Ok((Outbox::default(), None));
    }
    let (mut state, identity): (Outbox, _) =
        crate::paths::read_entry_guarded_with_identity(entry.reopen()?, || Ok(()))?;
    state.migrate();
    state.validate()?;
    Ok((state, Some(identity)))
}
