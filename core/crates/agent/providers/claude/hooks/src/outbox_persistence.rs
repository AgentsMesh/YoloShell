use super::types::OutboxState;
use super::*;
use crate::claude_hooks::{
    CLAUDE_HOOK_OUTBOX_SCHEMA_VERSION, CLAUDE_OUTBOX_LOCK_RETRY, CLAUDE_OUTBOX_LOCK_TIMEOUT,
    MAX_CLAUDE_OUTBOX_BYTES,
};
use anyhow::{Result, ensure};
use std::collections::VecDeque;

pub(super) fn update<T>(
    path: &Path,
    mutate: impl FnOnce(&mut OutboxState) -> Result<T>,
) -> Result<T> {
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(path)?;
    let _lock =
        entry.lock_parent_exclusive_wait(CLAUDE_OUTBOX_LOCK_TIMEOUT, CLAUDE_OUTBOX_LOCK_RETRY)?;
    let mut state = load(path)?;
    state.validate()?;
    let result = mutate(&mut state)?;
    state.validate()?;
    let bytes = serde_json::to_vec(&state)?;
    ensure!(
        bytes.len() as u64 <= MAX_CLAUDE_OUTBOX_BYTES,
        outbox_error::STATE_TOO_LARGE
    );
    #[cfg(test)]
    super::persistence_test_hook::run()?;
    crate::paths::atomic_write_json(path, &state)?;
    Ok(result)
}

pub(super) fn read<T>(path: &Path, inspect: impl FnOnce(&OutboxState) -> Result<T>) -> Result<T> {
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(path)?;
    let _lock =
        entry.lock_parent_exclusive_wait(CLAUDE_OUTBOX_LOCK_TIMEOUT, CLAUDE_OUTBOX_LOCK_RETRY)?;
    let state = load(path)?;
    state.validate()?;
    inspect(&state)
}

fn load(path: &Path) -> Result<OutboxState> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() == 0 => Ok(empty()),
        Ok(_) => crate::paths::read_json(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(empty()),
        Err(error) => Err(error.into()),
    }
}

fn empty() -> OutboxState {
    OutboxState {
        schema_version: CLAUDE_HOOK_OUTBOX_SCHEMA_VERSION,
        invocations: VecDeque::new(),
        failed_handshakes: VecDeque::new(),
    }
}
