use super::*;

impl CreationPhase {
    pub(in crate::host) fn permits_product_archive(self) -> bool {
        matches!(self, Self::Ready | Self::Terminated)
    }
}

impl CreationLedger {
    pub(in crate::host) fn active_creation(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<Option<(CreateTerminalResult, CreationPhase)>, ErrorBody> {
        self.ensure_healthy()?;
        Ok(self
            .bindings
            .iter()
            .find(|entry| entry.result.terminal_id == terminal_id)
            .map(|entry| (entry.result, entry.phase)))
    }

    pub(in crate::host) fn complete_product_archive(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_healthy()?;
        let mut candidate = self.clone();
        let Some(index) = candidate
            .bindings
            .iter()
            .position(|entry| entry.result == result)
        else {
            return Ok(());
        };
        if !candidate.bindings[index].phase.permits_product_archive() {
            return Err(ErrorBody::conflict(CREATION_PHASE_TRANSITION_CONFLICT));
        }
        let receipt =
            CreationReceipt::from_binding(&candidate.bindings[index], CreationCompletion::Archived);
        receipts.publish(&receipt).map_err(internal)?;
        candidate.bindings.remove(index);
        self.install(path, candidate)
    }
}
