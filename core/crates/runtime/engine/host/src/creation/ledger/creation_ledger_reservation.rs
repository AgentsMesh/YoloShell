use super::*;

impl CreationLedger {
    #[cfg(test)]
    pub(crate) fn reserve(
        &mut self,
        path: &Path,
        token: Uuid,
        spec: CreationSpec,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<CreationBinding, ErrorBody> {
        self.ensure_healthy()?;
        let binding = self.new_binding(creation_token_digest(token), spec, receipts)?;
        let mut candidate = self.clone();
        candidate.bindings.push(binding.clone());
        self.install(path, candidate)?;
        Ok(binding)
    }

    pub(crate) fn new_binding(
        &self,
        creation_token_digest: String,
        spec: CreationSpec,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<CreationBinding, ErrorBody> {
        if self.bindings.len() == MAX_CREATION_LEDGER_ENTRIES {
            return Err(ErrorBody::conflict("creation ledger capacity reached"));
        }
        let terminal_id = loop {
            let candidate = self.unique_uuid(|result| result.terminal_id);
            if receipts
                .lookup_session(candidate)
                .map_err(internal)?
                .is_none()
            {
                break candidate;
            }
        };
        Ok(CreationBinding {
            creation_token_digest,
            result: CreateTerminalResult {
                terminal_id: terminal_id,
                creation_capability: self.unique_uuid(|result| result.creation_capability),
            },
            spec,
            phase: CreationPhase::Reserved,
            launch_generation: RESERVED_CREATION_LAUNCH_GENERATION,
            launch: None,
        })
    }

    pub(crate) fn unique_uuid(&self, field: impl Fn(CreateTerminalResult) -> Uuid) -> Uuid {
        loop {
            let candidate = Uuid::new_v4();
            if !self
                .identities()
                .any(|(_, result)| field(result) == candidate)
            {
                return candidate;
            }
        }
    }
}
