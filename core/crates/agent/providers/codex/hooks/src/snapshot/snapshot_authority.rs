use super::CodexInvocationAuthority;
use crate::paths::identity::{FileIdentity, ObjectIdentity};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodexSnapshotLineage {
    directory: ObjectIdentity,
    lock: ObjectIdentity,
    snapshot: FileIdentity,
}

impl CodexSnapshotLineage {
    pub(super) fn require_invocation(self, authority: CodexInvocationAuthority) -> Result<()> {
        authority.require_directory(self.directory)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SnapshotAuthority {
    directory: ObjectIdentity,
    lock: ObjectIdentity,
}

impl SnapshotAuthority {
    pub(super) fn new(directory: ObjectIdentity, lock: ObjectIdentity) -> Self {
        Self { directory, lock }
    }

    pub(super) fn lineage(self, snapshot: Option<FileIdentity>) -> Result<CodexSnapshotLineage> {
        let snapshot = snapshot.ok_or_else(super::snapshot_error::snapshot_changed)?;
        Ok(CodexSnapshotLineage {
            directory: self.directory,
            lock: self.lock,
            snapshot,
        })
    }

    pub(super) fn require(self, expected: Self) -> Result<()> {
        if self.directory != expected.directory {
            return Err(super::snapshot_error::directory_changed());
        }
        if self.lock != expected.lock {
            return Err(super::snapshot_error::lock_changed());
        }
        Ok(())
    }

    pub(super) fn require_lock(self, observed: Option<ObjectIdentity>) -> Result<()> {
        if observed != Some(self.lock) {
            return Err(super::snapshot_error::lock_changed());
        }
        Ok(())
    }

    pub(super) fn require_lineage(
        self,
        snapshot: Option<FileIdentity>,
        expected: CodexSnapshotLineage,
    ) -> Result<()> {
        self.require(Self::new(expected.directory, expected.lock))?;
        if snapshot != Some(expected.snapshot) {
            return Err(super::snapshot_error::snapshot_changed());
        }
        Ok(())
    }
}
