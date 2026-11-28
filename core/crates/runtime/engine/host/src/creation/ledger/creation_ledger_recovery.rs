use super::*;

impl CreationLedger {
    pub(in crate::host) fn recover_completed(
        &mut self,
        path: &Path,
        receipts: &CreationReceiptStore,
    ) -> Result<()> {
        self.validate()?;
        let mut candidate = self.clone();
        let mut completed = Vec::new();
        for binding in &candidate.bindings {
            let Some(receipt) = receipts.lookup_binding(binding)? else {
                continue;
            };
            anyhow::ensure!(
                !matches!(
                    receipt.completion,
                    CreationCompletion::Finalized | CreationCompletion::Survivor
                ) || binding.phase == CreationPhase::Ready,
                CREATION_LIVE_RECEIPT_PHASE_CONFLICT
            );
            receipts.publish(&receipt)?;
            completed.push(binding.result);
        }
        if completed.is_empty() {
            return Ok(());
        }
        candidate
            .bindings
            .retain(|binding| !completed.contains(&binding.result));
        candidate.validate()?;
        match &self.entry {
            Some(entry) => write_creation_ledger_entry(entry, &candidate)?,
            None => write_creation_ledger(path, &candidate)?,
        }
        *self = candidate;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn recover_token_completion(
        &mut self,
        path: &Path,
        receipts: &CreationReceiptStore,
        token: Uuid,
    ) -> std::result::Result<(), ErrorBody> {
        let digest = creation_token_digest(token);
        let index = self
            .bindings
            .iter()
            .position(|binding| binding.creation_token_digest == digest);
        self.recover_binding_completion(path, receipts, index)
            .map(|_| ())
    }

    pub(crate) fn recover_session_completion(
        &mut self,
        path: &Path,
        receipts: &CreationReceiptStore,
        terminal_id: Uuid,
    ) -> std::result::Result<Option<CreationCompletion>, ErrorBody> {
        let index = self
            .bindings
            .iter()
            .position(|binding| binding.result.terminal_id == terminal_id);
        self.recover_binding_completion(path, receipts, index)
    }

    pub(crate) fn recover_binding_completion(
        &mut self,
        path: &Path,
        receipts: &CreationReceiptStore,
        index: Option<usize>,
    ) -> std::result::Result<Option<CreationCompletion>, ErrorBody> {
        self.ensure_healthy()?;
        let Some(index) = index else { return Ok(None) };
        let binding = &self.bindings[index];
        let Some(receipt) = receipts.lookup_binding(binding).map_err(internal)? else {
            return Ok(None);
        };
        if matches!(
            receipt.completion,
            CreationCompletion::Finalized | CreationCompletion::Survivor
        ) && binding.phase != CreationPhase::Ready
        {
            return Err(ErrorBody::internal(CREATION_LIVE_RECEIPT_PHASE_CONFLICT));
        }
        receipts.publish(&receipt).map_err(internal)?;
        let mut candidate = self.clone();
        candidate.bindings.remove(index);
        self.install(path, candidate)?;
        Ok(Some(receipt.completion))
    }
}
