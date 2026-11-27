use super::snapshot_authority::SnapshotAuthority;
use super::snapshot_cursor::{CodexSnapshotCursor, CodexSnapshotProof};
use super::snapshot_lock::SnapshotLock;
use super::{snapshot_error, snapshot_storage};
use crate::agent::hooks::ReducerSnapshot;
use crate::paths::directory::PrivateDirectory;
use crate::paths::identity::FileIdentity;
use anyhow::Result;
use std::cell::Cell;
use std::path::PathBuf;

pub struct CodexSnapshotClaim<'cursor> {
    cursor: &'cursor mut CodexSnapshotCursor,
    next_cursor: CodexSnapshotCursor,
    proof: CodexSnapshotProof,
    token: CodexSnapshotClaimToken,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodexSnapshotClaimToken(FileIdentity);

impl CodexSnapshotClaim<'_> {
    pub fn publish_with(
        self,
        publish: impl FnOnce(&CodexSnapshotProof) -> Result<()>,
    ) -> Result<CodexSnapshotClaimToken> {
        let Self {
            cursor,
            next_cursor,
            proof,
            token,
        } = self;
        publish(&proof)?;
        drop(proof);
        *cursor = next_cursor;
        Ok(token)
    }
}

impl CodexSnapshotClaimToken {
    pub(super) fn matches(&self, expected: Option<FileIdentity>) -> bool {
        expected == Some(self.0)
    }
}

pub(super) fn claim<'cursor>(
    cursor: &'cursor mut CodexSnapshotCursor,
    directory: PrivateDirectory,
    path: PathBuf,
    expected: Option<FileIdentity>,
    authority: SnapshotAuthority,
    initial: &ReducerSnapshot,
) -> Result<CodexSnapshotClaim<'cursor>> {
    let lock = SnapshotLock::acquire_expected(&directory, &path, authority)?;
    require_expected(&lock, expected)?;
    let loaded = snapshot_storage::load(&lock, &path)?;
    require_loaded_identity(loaded.identity, expected)?;
    let value = loaded.snapshot.as_ref().unwrap_or(initial);
    let encoded = snapshot_storage::encode(value)?;
    let committed = Cell::new(None);
    let result = crate::paths::write_expected_entry_guarded_with_commit_observer(
        lock.entry(&path)?,
        expected,
        &encoded,
        || lock.verify(),
        || {
            run_before_claim_commit_hook();
            snapshot_storage::prove_current(&lock, &path, expected)
        },
        |identity| {
            committed.set(Some(identity));
            run_after_claim_rename_hook();
        },
    );
    let identity = match result {
        Ok(identity) => identity,
        Err(error) => {
            return Err(snapshot_storage::classify_commit_failure(
                &lock,
                &path,
                expected,
                committed.get(),
                error,
            ));
        }
    };
    snapshot_storage::prove_retained_current(&lock, &path, identity)?;
    Ok(build_claim(cursor, directory, path, lock, identity))
}

fn build_claim<'cursor>(
    cursor: &'cursor mut CodexSnapshotCursor,
    directory: PrivateDirectory,
    path: PathBuf,
    lock: SnapshotLock,
    identity: FileIdentity,
) -> CodexSnapshotClaim<'cursor> {
    let authority = lock.authority();
    CodexSnapshotClaim {
        cursor,
        next_cursor: CodexSnapshotCursor::from_claim(directory, path.clone(), identity, authority),
        proof: CodexSnapshotProof::retained_claim(lock, path, identity),
        token: CodexSnapshotClaimToken(identity),
    }
}

fn require_expected(lock: &SnapshotLock, expected: Option<FileIdentity>) -> Result<()> {
    if lock.target_identity() != expected {
        return Err(snapshot_error::snapshot_changed());
    }
    Ok(())
}

fn require_loaded_identity(
    loaded: Option<FileIdentity>,
    expected: Option<FileIdentity>,
) -> Result<()> {
    if loaded != expected {
        return Err(snapshot_error::snapshot_changed());
    }
    Ok(())
}

#[cfg(any(test, feature = "runtime-component-tests"))]
thread_local! {
    static BEFORE_CLAIM_COMMIT_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static AFTER_CLAIM_RENAME_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn set_before_claim_commit_hook(hook: impl FnOnce() + 'static) {
    BEFORE_CLAIM_COMMIT_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn set_after_claim_rename_hook(hook: impl FnOnce() + 'static) {
    AFTER_CLAIM_RENAME_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn run_before_claim_commit_hook() {
    run_hook(&BEFORE_CLAIM_COMMIT_HOOK);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn run_after_claim_rename_hook() {
    run_hook(&AFTER_CLAIM_RENAME_HOOK);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn run_hook(slot: &'static std::thread::LocalKey<std::cell::RefCell<Option<Box<dyn FnOnce()>>>>) {
    let hook = slot.with(|value| value.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
fn run_before_claim_commit_hook() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
fn run_after_claim_rename_hook() {}
