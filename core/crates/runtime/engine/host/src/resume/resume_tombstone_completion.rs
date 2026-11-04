use super::*;

impl Host {
    pub(crate) fn consume_resume_tombstone(&self, transaction: &ResumeTransaction) -> Result<()> {
        #[cfg(test)]
        if let Some(boundary) = lock_recover(&self.catalog_test_probes.tombstone_consume).take() {
            return self.consume_resume_tombstone_with_boundary(transaction, boundary);
        }
        self.consume_resume_tombstone_with_boundary(transaction, || Ok(()))
    }

    pub(crate) fn consume_resume_tombstone_with_boundary<F>(
        &self,
        transaction: &ResumeTransaction,
        boundary: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        if transaction.intent != ResumeIntent::Explicit {
            return Ok(());
        }
        let Some(tombstone) = self.resume_tombstone(transaction)? else {
            return Ok(());
        };
        if self.resume_tombstone_is_planned(transaction, &tombstone)? {
            return Ok(());
        }
        let path = self.paths.terminate_tombstone(transaction.terminal_id);
        match self
            .namespace
            .remove_json_if_matches_with_boundary(&path, &tombstone, boundary)
        {
            Ok(()) => Ok(()),
            Err(error) => self.settle_changed_resume_tombstone(transaction, error),
        }
    }

    pub(crate) fn settle_changed_resume_tombstone(
        &self,
        transaction: &ResumeTransaction,
        first_error: anyhow::Error,
    ) -> Result<()> {
        let Some(current) = self.resume_tombstone(transaction)? else {
            return Ok(());
        };
        if self.resume_tombstone_is_planned(transaction, &current)? {
            return Ok(());
        }
        let path = self.paths.terminate_tombstone(transaction.terminal_id);
        match self.namespace.remove_json_if_matches(&path, &current) {
            Ok(()) => Ok(()),
            Err(retry_error) => match self.resume_tombstone(transaction)? {
                None => Ok(()),
                Some(final_tombstone)
                    if self.resume_tombstone_is_planned(transaction, &final_tombstone)? =>
                {
                    Ok(())
                }
                Some(_) => Err(retry_error.context(first_error.to_string())),
            },
        }
    }

    pub(crate) fn resume_tombstone(
        &self,
        transaction: &ResumeTransaction,
    ) -> Result<Option<crate::model::TerminationTombstone>> {
        let path = self.paths.terminate_tombstone(transaction.terminal_id);
        if self.namespace.file_is_missing(&path)? {
            return Ok(None);
        }
        let tombstone: crate::model::TerminationTombstone = self.read_namespace_json(&path)?;
        let previous = tombstone
            .validate_for(transaction.terminal_id, transaction.previous_runtime_epoch)
            .is_ok();
        if !previous && !self.resume_tombstone_is_planned(transaction, &tombstone)? {
            bail!(RESUME_TOMBSTONE_MISMATCH);
        }
        Ok(Some(tombstone))
    }

    pub(crate) fn resume_tombstone_is_planned(
        &self,
        transaction: &ResumeTransaction,
        tombstone: &crate::model::TerminationTombstone,
    ) -> Result<bool> {
        if tombstone
            .validate_for(transaction.terminal_id, transaction.planned_runtime_epoch)
            .is_ok()
        {
            return Ok(true);
        }
        tombstone
            .validate_for(transaction.terminal_id, transaction.previous_runtime_epoch)
            .map(|()| false)
            .map_err(|error| anyhow::anyhow!("{RESUME_TOMBSTONE_MISMATCH}: {error}"))
    }
}
