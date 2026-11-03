use super::*;
use crate::paths::directory::{DirectoryRevision, PrivateDirectory};
use anyhow::Result;
use std::path::PathBuf;
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};
#[path = "scavenge_candidate.rs"]
mod candidate;
#[cfg(test)]
pub(super) use candidate::changed_or_unproven;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../test_support/unit/coordination/scavenge_test_support.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub(super) use test_support::{sweep, sweep_for_test, sweep_pass_for_test};

const SCANNER_LABEL: &str = "coordination";

#[derive(Default)]
pub(super) struct SweepWork {
    pub(super) inspected: usize,
    pub(super) probed: usize,
    pub(super) deleted: usize,
}

pub struct CoordinationScanner {
    root_path: PathBuf,
    root: Option<PrivateDirectory>,
    stream: Option<stream::DirectoryStream>,
    revision: Option<DirectoryRevision>,
}

impl CoordinationScanner {
    #[cfg(not(test))]
    pub fn new() -> Self {
        Self {
            root_path: coordination_root(),
            root: None,
            stream: None,
            revision: None,
        }
    }

    #[cfg(test)]
    pub(super) fn from_root(root: PrivateDirectory) -> Self {
        Self {
            root_path: root.path().to_path_buf(),
            root: Some(root),
            stream: None,
            revision: None,
        }
    }

    #[cfg(test)]
    pub(super) fn from_path(root_path: PathBuf) -> Self {
        Self {
            root_path,
            root: None,
            stream: None,
            revision: None,
        }
    }

    fn ensure_root(&mut self) -> Result<()> {
        if self.root.is_none() {
            self.root = Some(PrivateDirectory::ensure(&self.root_path)?);
        }
        Ok(())
    }

    fn reset_enumeration(&mut self) {
        self.stream = None;
        self.revision = None;
    }

    pub(super) fn scan_with_limits(
        &mut self,
        entry_budget: usize,
        delete_budget: usize,
    ) -> Result<(ScanChunk, SweepWork)> {
        self.scan_with_limits_using(entry_budget, delete_budget, PrivateDirectory::revision)
    }

    fn scan_with_limits_using<F>(
        &mut self,
        entry_budget: usize,
        delete_budget: usize,
        after_revision: F,
    ) -> Result<(ScanChunk, SweepWork)>
    where
        F: FnOnce(&PrivateDirectory) -> Result<DirectoryRevision>,
    {
        self.ensure_root()?;
        let root = self.root.as_ref().expect("root initialized");
        let Some(_lifecycle) = lifecycle::LifecycleLock::try_acquire(root)? else {
            return Ok((ScanChunk::Busy, SweepWork::default()));
        };
        root.verify_path()?;
        let before = root.revision()?;
        if self.revision != Some(before) || self.stream.is_none() {
            self.stream = Some(stream::DirectoryStream::open(root.descriptor())?);
            self.revision = Some(before);
        }
        let result = self.scan_locked(entry_budget, delete_budget, after_revision);
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn scan_locked(
        &mut self,
        entry_budget: usize,
        delete_budget: usize,
        after_revision: impl FnOnce(&PrivateDirectory) -> Result<DirectoryRevision>,
    ) -> Result<(ScanChunk, SweepWork)> {
        let root = self.root.as_ref().expect("root initialized");
        let mut work = SweepWork::default();
        let mut complete = false;
        let mut mutated = false;
        for _ in 0..entry_budget {
            let Some(name) = self
                .stream
                .as_mut()
                .expect("stream initialized")
                .next_name()?
            else {
                complete = true;
                break;
            };
            work.inspected += 1;
            if work.deleted >= delete_budget || !candidate::is_lock_name(&name) {
                continue;
            }
            work.probed += 1;
            if candidate::clean(root, &name)? {
                work.deleted += 1;
                mutated = true;
                break;
            }
        }
        let after = after_revision(root)?;
        if mutated || self.revision != Some(after) {
            self.reset_enumeration();
            self.revision = Some(after);
            return Ok((ScanChunk::Progress, work));
        }
        if complete {
            self.reset_enumeration();
            return Ok((ScanChunk::PassComplete, work));
        }
        self.revision = Some(after);
        Ok((ScanChunk::Progress, work))
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/scavenge_branch_tests.rs"]
mod branch_tests;

impl MaintenanceScanner for CoordinationScanner {
    fn label(&self) -> &'static str {
        SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.scan_with_limits(SWEEP_ENTRY_BUDGET, SWEEP_DELETE_BUDGET)
            .map(|value| value.0)
    }

    fn reset(&mut self) {
        self.reset_enumeration();
        self.root = None;
    }
}
