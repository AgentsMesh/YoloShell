use super::snapshot_claim;
use super::snapshot_lock::{SnapshotLock, validate_path};
use super::snapshot_storage;
use super::{
    CodexSnapshotClaim, CodexSnapshotClaimToken, CodexSnapshotLineage,
    snapshot_authority::SnapshotAuthority,
};
use crate::agent::hooks::ReducerSnapshot;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[path = "snapshot_terminal_proof.rs"]
mod terminal_proof;

pub struct CodexSnapshotCursor {
    directory: crate::paths::directory::PrivateDirectory,
    path: PathBuf,
    expected: Option<FileIdentity>,
    authority: SnapshotAuthority,
}

pub struct CodexSnapshotProof {
    lock: SnapshotLock,
    path: PathBuf,
    basis: SnapshotProofBasis,
}

#[derive(Clone, Copy)]
enum SnapshotProofBasis {
    AcquisitionBaseline(Option<FileIdentity>),
    RetainedClaim(FileIdentity),
}

impl CodexSnapshotCursor {
    pub fn open(path: impl Into<PathBuf>) -> Result<(Self, Option<ReducerSnapshot>)> {
        let path = path.into();
        validate_path(&path)?;
        let directory = crate::paths::secure_private_parent(&path)
            .context(super::constants::errors::SNAPSHOT_PATH_INVALID)?;
        Self::open_in(&directory, path)
    }

    pub(crate) fn open_in(
        directory: &crate::paths::directory::PrivateDirectory,
        path: impl Into<PathBuf>,
    ) -> Result<(Self, Option<ReducerSnapshot>)> {
        let path = path.into();
        validate_path(&path)?;
        anyhow::ensure!(
            path.parent() == Some(directory.path()),
            super::constants::errors::SNAPSHOT_PATH_INVALID
        );
        let lock = SnapshotLock::acquire(directory, &path)?;
        let loaded = snapshot_storage::load(&lock, &path)?;
        Ok((
            Self {
                directory: directory.try_clone()?,
                path,
                expected: loaded.identity,
                authority: lock.authority(),
            },
            loaded.snapshot,
        ))
    }

    pub fn persist(&self, snapshot: &ReducerSnapshot) -> Result<Self> {
        let next_directory = self.directory.try_clone()?;
        let lock = SnapshotLock::acquire_expected(&self.directory, &self.path, self.authority)?;
        let expected = snapshot_storage::persist(&lock, &self.path, self.expected, snapshot)?;
        Ok(Self {
            directory: next_directory,
            path: self.path.clone(),
            expected: Some(expected),
            authority: self.authority,
        })
    }

    pub fn claim(&mut self, initial: &ReducerSnapshot) -> Result<CodexSnapshotClaim<'_>> {
        let next_directory = self
            .directory
            .try_clone()
            .context(super::constants::errors::SNAPSHOT_LOCK)?;
        let path = self.path.clone();
        let expected = self.expected;
        let authority = self.authority;
        snapshot_claim::claim(self, next_directory, path, expected, authority, initial)
    }

    pub fn prove_current(&self) -> Result<CodexSnapshotProof> {
        let lock = SnapshotLock::acquire_expected(&self.directory, &self.path, self.authority)?;
        snapshot_storage::prove_current(&lock, &self.path, self.expected)?;
        Ok(CodexSnapshotProof {
            lock,
            path: self.path.clone(),
            basis: SnapshotProofBasis::AcquisitionBaseline(self.expected),
        })
    }

    pub(crate) fn is_claimed_by(&self, token: &CodexSnapshotClaimToken) -> bool {
        token.matches(self.expected)
    }

    pub fn snapshot_lineage(&self) -> Result<CodexSnapshotLineage> {
        self.authority.lineage(self.expected)
    }

    pub fn require_lineage(&self, expected: CodexSnapshotLineage) -> Result<()> {
        self.authority.require_lineage(self.expected, expected)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn from_claim(
        directory: crate::paths::directory::PrivateDirectory,
        path: PathBuf,
        identity: FileIdentity,
        authority: SnapshotAuthority,
    ) -> Self {
        Self {
            directory,
            path,
            expected: Some(identity),
            authority,
        }
    }
}

impl CodexSnapshotProof {
    pub(super) fn retained_claim(
        lock: SnapshotLock,
        path: PathBuf,
        identity: FileIdentity,
    ) -> Self {
        Self {
            lock,
            path,
            basis: SnapshotProofBasis::RetainedClaim(identity),
        }
    }

    pub fn linearize_publication(&self) -> Result<()> {
        run_before_publication_hook();
        self.verify_current()
    }

    fn verify_current(&self) -> Result<()> {
        match self.basis {
            SnapshotProofBasis::AcquisitionBaseline(expected) => {
                snapshot_storage::prove_current(&self.lock, &self.path, expected)
            }
            SnapshotProofBasis::RetainedClaim(expected) => {
                snapshot_storage::prove_retained_current(&self.lock, &self.path, expected)
            }
        }
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
thread_local! {
    static BEFORE_PUBLICATION_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn set_before_publication_hook(hook: impl FnOnce() + 'static) {
    BEFORE_PUBLICATION_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn run_before_publication_hook() {
    let hook = BEFORE_PUBLICATION_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
fn run_before_publication_hook() {}
