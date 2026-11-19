use super::super::*;
use super::{
    ISOLATION_MISMATCH_ERROR, ISOLATION_NOT_TRUNCATED_ERROR, ISOLATION_RECLAIM_ERROR,
    SOURCE_REPLACED_ERROR,
};

#[derive(Clone, Copy)]
enum IsolationRequirement {
    Truncated,
    AnyLength,
}

impl PrivateFileLock {
    pub fn reclaim_isolation_of(&mut self, source: &PrivateFileEntry) -> Result<()> {
        let isolation = self.require_isolation_of(source, IsolationRequirement::Truncated)?;
        anyhow::ensure!(source.is_missing()?, SOURCE_REPLACED_ERROR);
        anyhow::ensure!(
            source.remove_if_exact(self.identity, self.max_bytes)?,
            ISOLATION_RECLAIM_ERROR
        );
        anyhow::ensure!(
            source.is_missing()? && isolation.is_missing()?,
            ISOLATION_RECLAIM_ERROR
        );
        Ok(())
    }

    pub(crate) fn reclaim_isolation_preserving_source(
        &mut self,
        source: &PrivateFileEntry,
    ) -> Result<()> {
        self.reclaim_isolation_preserving_source_with(source, IsolationRequirement::Truncated)
    }

    pub(crate) fn reclaim_unlink_isolation_of(&mut self, source: &PrivateFileEntry) -> Result<()> {
        self.reclaim_isolation_preserving_source_with(source, IsolationRequirement::AnyLength)
    }

    fn reclaim_isolation_preserving_source_with(
        &mut self,
        source: &PrivateFileEntry,
        requirement: IsolationRequirement,
    ) -> Result<()> {
        let isolation = self.require_isolation_of(source, requirement)?;
        let source_identity = source.inspect(self.max_bytes)?;
        anyhow::ensure!(
            source_identity.is_none_or(|identity| !identity.same_object(self.identity)),
            SOURCE_REPLACED_ERROR
        );
        let proof = self.capture_unlink_proof()?;
        self.entry
            .unlink_exact_deferred(self.identity, self.max_bytes)?;
        self.verify_unlinked(proof)?;
        self.entry.sync_parent()?;
        match source_identity {
            Some(identity) => source.verify_current_unchanged(identity, self.max_bytes)?,
            None => anyhow::ensure!(source.is_missing()?, SOURCE_REPLACED_ERROR),
        }
        anyhow::ensure!(isolation.is_missing()?, ISOLATION_RECLAIM_ERROR);
        Ok(())
    }

    fn require_isolation_of(
        &self,
        source: &PrivateFileEntry,
        requirement: IsolationRequirement,
    ) -> Result<PrivateFileEntry> {
        self.verify_descriptor()?;
        let isolation = source.unlink_isolation_entry()?;
        anyhow::ensure!(
            self.entry.path() == isolation.path(),
            ISOLATION_MISMATCH_ERROR
        );
        if matches!(requirement, IsolationRequirement::Truncated) {
            anyhow::ensure!(
                self.identity.length() == u64::default(),
                ISOLATION_NOT_TRUNCATED_ERROR
            );
        }
        Ok(isolation)
    }
}
