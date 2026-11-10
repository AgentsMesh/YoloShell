use super::*;
use std::ffi::CStr;

#[path = "private_file_lock_isolation_recovery.rs"]
mod isolation_recovery;

const LOCKED_FILE_CHANGED_ERROR: &str = "locked private file changed";
const QUARANTINE_OCCUPIED_ERROR: &str = "private reclaim quarantine is occupied";
const SOURCE_REPLACED_ERROR: &str = "private reclaim source was replaced";
const ISOLATION_MISMATCH_ERROR: &str = "private reclaim isolation entry mismatch";
const ISOLATION_NOT_TRUNCATED_ERROR: &str = "private reclaim isolation is not truncated";
const ISOLATION_RECLAIM_ERROR: &str = "private reclaim isolation was not removed";
const SYNC_RECLAIM_CONTEXT: &str = "sync locked private reclaim file";
const TRUNCATE_RECLAIM_CONTEXT: &str = "truncate locked private reclaim file";

impl PrivateFileLock {
    pub fn verify(&self) -> Result<()> {
        self.verify_descriptor()
    }

    pub(crate) fn unlink_owned(&mut self) -> Result<()> {
        self.unlink_owned_with(|| Ok(()))
    }

    pub fn unlink_owned_between(
        &mut self,
        active_name: &CStr,
        isolation_name: &CStr,
    ) -> Result<()> {
        self.unlink_owned_between_with(active_name, isolation_name, || Ok(()))
    }

    pub fn reclaim_owned(&mut self) -> Result<()> {
        self.verify_descriptor()?;
        self.truncate_and_unlink(|| Ok(()))
    }

    fn unlink_owned_with<F>(&mut self, hook: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.verify_descriptor()?;
        let proof = self.capture_unlink_proof()?;
        self.entry.unlink_named_identity(
            self.entry.file_name(),
            self.identity,
            self.max_bytes,
            hook,
        )?;
        self.verify_unlinked(proof)
    }

    fn unlink_owned_between_with<F>(
        &mut self,
        active_name: &CStr,
        isolation_name: &CStr,
        after_isolation: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.verify_descriptor()?;
        anyhow::ensure!(active_name != isolation_name, ISOLATION_MISMATCH_ERROR);
        let active = self.entry.sibling_named(active_name)?;
        if self.entry.file_name() == active_name {
            let isolated = self.entry.sibling_named(isolation_name)?;
            anyhow::ensure!(isolated.is_missing()?, QUARANTINE_OCCUPIED_ERROR);
            self.entry.quarantine_named_identity(
                active_name,
                isolation_name,
                self.identity,
                self.max_bytes,
            )?;
            self.entry = isolated;
            self.identity =
                FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), self.max_bytes)?;
        } else {
            anyhow::ensure!(
                self.entry.file_name() == isolation_name,
                ISOLATION_MISMATCH_ERROR
            );
        }
        after_isolation()?;
        self.verify_descriptor()?;
        anyhow::ensure!(active.is_missing()?, SOURCE_REPLACED_ERROR);
        let proof = self.capture_unlink_proof()?;
        self.entry
            .unlink_exact_deferred(self.identity, self.max_bytes)?;
        self.verify_unlinked(proof)?;
        self.entry.sync_parent()?;
        anyhow::ensure!(active.is_missing()?, SOURCE_REPLACED_ERROR);
        anyhow::ensure!(self.entry.is_missing()?, ISOLATION_RECLAIM_ERROR);
        Ok(())
    }

    pub(in crate::paths) fn quarantine_and_unlink<F, G, H>(
        &mut self,
        quarantine_name: &std::ffi::CStr,
        before_quarantine: F,
        after_quarantine: G,
        before_unlink: H,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
        G: FnOnce() -> Result<()>,
        H: FnOnce() -> Result<()>,
    {
        self.verify_descriptor()?;
        before_quarantine()?;
        self.verify_descriptor()?;
        let quarantine = self.entry.sibling_named(quarantine_name)?;
        anyhow::ensure!(quarantine.is_missing()?, QUARANTINE_OCCUPIED_ERROR);
        self.entry.quarantine_named_identity(
            self.entry.file_name(),
            quarantine_name,
            self.identity,
            self.max_bytes,
        )?;
        after_quarantine()?;
        let source_replaced = !self.entry.is_missing()?;
        self.entry = quarantine;
        self.truncate_and_unlink(before_unlink)?;
        anyhow::ensure!(!source_replaced, SOURCE_REPLACED_ERROR);
        Ok(())
    }

    pub(in crate::paths) fn truncate_and_unlink<F>(&mut self, before_unlink: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.file.set_len(0).context(TRUNCATE_RECLAIM_CONTEXT)?;
        self.file.sync_all().context(SYNC_RECLAIM_CONTEXT)?;
        self.identity =
            FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), self.max_bytes)?;
        let proof = self.capture_unlink_proof()?;
        self.entry.unlink_named_identity(
            self.entry.file_name(),
            self.identity,
            self.max_bytes,
            before_unlink,
        )?;
        self.verify_unlinked(proof)
    }

    fn verify_descriptor(&self) -> Result<()> {
        let current =
            FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), self.max_bytes)?;
        anyhow::ensure!(current.unchanged(self.identity), LOCKED_FILE_CHANGED_ERROR);
        self.entry
            .verify_current_unchanged(self.identity, self.max_bytes)
    }

    #[cfg(test)]
    pub(crate) fn unlink_owned_with_hook<F>(&mut self, hook: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.unlink_owned_with(hook)
    }

    #[cfg(any(test, feature = "persistence-test-support"))]
    pub(crate) fn unlink_owned_between_with_hook<F>(
        &mut self,
        active_name: &CStr,
        isolation_name: &CStr,
        hook: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.unlink_owned_between_with(active_name, isolation_name, hook)
    }
}
