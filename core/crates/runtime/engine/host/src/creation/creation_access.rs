use super::*;

impl Host {
    #[cfg(test)]
    pub(crate) fn existing_creation(
        &self,
        path: &Path,
        token: Uuid,
        fingerprint: &str,
    ) -> std::result::Result<Option<CreationBinding>, ErrorBody> {
        let mut ledger = lock_recover(&self.creation_capabilities);
        ledger.recover_token_completion(path, &self.creation_receipts, token)?;
        ledger.existing(token, fingerprint, &self.creation_receipts)
    }

    #[cfg(test)]
    pub(crate) fn reserve_creation(
        &self,
        path: &Path,
        token: Uuid,
        spec: CreationSpec,
    ) -> std::result::Result<CreationBinding, ErrorBody> {
        let mut ledger = lock_recover(&self.creation_capabilities);
        ledger.recover_token_completion(path, &self.creation_receipts, token)?;
        if let Some(binding) =
            ledger.existing(token, &spec.request_fingerprint, &self.creation_receipts)?
        {
            return Ok(binding);
        }
        ledger.reserve(path, token, spec, &self.creation_receipts)
    }

    pub(in crate::host) fn recover_session_creation(
        &self,
        path: &Path,
        terminal_id: Uuid,
    ) -> std::result::Result<Option<CreationCompletion>, ErrorBody> {
        lock_recover(&self.creation_capabilities).recover_session_completion(
            path,
            &self.creation_receipts,
            terminal_id,
        )
    }
}
