use super::*;

impl CreationLedger {
    pub(in crate::host) fn finalize(
        &mut self,
        path: &Path,
        request: FinalizeCreationRequest,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_healthy()?;
        let digest = creation_token_digest(request.creation_token);
        let result = CreateTerminalResult {
            terminal_id: request.terminal_id,
            creation_capability: request.creation_capability,
        };
        match receipts
            .lookup_identity(&digest, result)
            .map_err(internal)?
        {
            CreationReceiptLookup::Matched(receipt) => {
                self.validate_finalize_receipt_fingerprint(&receipt)?;
                return if matches!(
                    receipt.completion,
                    CreationCompletion::Finalized | CreationCompletion::Survivor
                ) {
                    receipts.publish(&receipt).map_err(internal)?;
                    self.remove_completed_binding(path, result)?;
                    Ok(())
                } else {
                    Err(creation_unauthorized())
                };
            }
            CreationReceiptLookup::Mismatched => return Err(creation_unauthorized()),
            CreationReceiptLookup::Missing => {}
        }
        let mut candidate = self.clone();
        let index = candidate
            .bindings
            .iter()
            .position(|entry| entry.creation_token_digest == digest)
            .ok_or_else(creation_unauthorized)?;
        let binding = &candidate.bindings[index];
        if binding.result != result {
            return Err(creation_unauthorized());
        }
        if binding.phase != CreationPhase::Ready {
            return Err(ErrorBody::conflict(
                "only a ready creation can be finalized",
            ));
        }
        let receipt = CreationReceipt::from_binding(
            &candidate.bindings[index],
            CreationCompletion::Finalized,
        );
        receipts.publish(&receipt).map_err(internal)?;
        candidate.bindings.remove(index);
        self.install(path, candidate)
    }

    pub(in crate::host) fn pending_finalize(
        &self,
        request: &FinalizeCreationRequest,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<Option<CreationBinding>, ErrorBody> {
        self.ensure_healthy()?;
        let digest = creation_token_digest(request.creation_token);
        let result = CreateTerminalResult {
            terminal_id: request.terminal_id,
            creation_capability: request.creation_capability,
        };
        match receipts
            .lookup_identity(&digest, result)
            .map_err(internal)?
        {
            CreationReceiptLookup::Matched(receipt) => {
                self.validate_finalize_receipt_fingerprint(&receipt)?;
                return if matches!(
                    receipt.completion,
                    CreationCompletion::Finalized | CreationCompletion::Survivor
                ) {
                    Ok(None)
                } else {
                    Err(creation_unauthorized())
                };
            }
            CreationReceiptLookup::Mismatched => return Err(creation_unauthorized()),
            CreationReceiptLookup::Missing => {}
        }
        let binding = self
            .bindings
            .iter()
            .find(|entry| entry.creation_token_digest == digest)
            .ok_or_else(creation_unauthorized)?;
        if binding.result != result {
            return Err(creation_unauthorized());
        }
        if binding.phase != CreationPhase::Ready {
            return Err(ErrorBody::conflict(
                "only a ready creation can be finalized",
            ));
        }
        Ok(Some(binding.clone()))
    }

    pub(crate) fn validate_finalize_receipt_fingerprint(
        &self,
        receipt: &CreationReceipt,
    ) -> std::result::Result<(), ErrorBody> {
        if let Some(binding) = self
            .bindings
            .iter()
            .find(|binding| binding.creation_token_digest == receipt.creation_token_digest)
        {
            receipt
                .validate_binding_fingerprint(binding)
                .map_err(internal)?;
        }
        Ok(())
    }

    pub(crate) fn remove_completed_binding(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
    ) -> std::result::Result<(), ErrorBody> {
        let mut candidate = self.clone();
        candidate
            .bindings
            .retain(|binding| binding.result != result);
        if candidate.bindings.len() == self.bindings.len() {
            return Ok(());
        }
        self.install(path, candidate)
    }
}
