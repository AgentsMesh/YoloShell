impl PtySessionProof {
    pub(in crate::process) fn mark_reopen_seal_destructive_fence_crossed(&mut self) -> Result<()> {
        Ok(())
    }

    pub(in crate::process) fn commit_reopen_seal_for_destruction(&mut self) -> Result<()> {
        Ok(())
    }

    pub(in crate::process) fn settle_reopen_seal_after_fence(&mut self) -> Result<()> {
        Ok(())
    }

    pub(in crate::process) fn reopen_seal_commit_requires_fresh_eof(&self) -> bool {
        false
    }

    pub(in crate::process) fn rollback_reopen_seal_before_commit(&mut self) -> Result<()> {
        Ok(())
    }

    pub(in crate::process) fn quarantine_reopen_seal(&mut self) -> Result<()> {
        Ok(())
    }
}
