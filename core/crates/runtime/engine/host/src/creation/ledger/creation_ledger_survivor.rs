use super::*;

impl CreationLedger {
    pub(in crate::host) fn complete_survivor(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_healthy()?;
        let mut candidate = self.clone();
        let index = candidate
            .bindings
            .iter()
            .position(|entry| entry.result == result)
            .ok_or_else(creation_unauthorized)?;
        if candidate.bindings[index].phase != CreationPhase::Ready {
            return Err(ErrorBody::conflict(CREATION_SURVIVOR_PHASE_CONFLICT));
        }
        let receipt =
            CreationReceipt::from_binding(&candidate.bindings[index], CreationCompletion::Survivor);
        receipts.publish(&receipt).map_err(internal)?;
        candidate.bindings.remove(index);
        self.install(path, candidate)
    }
}
