use super::*;

impl CreationLedger {
    #[cfg(test)]
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let entry = crate::paths::private_file::PrivateFileEntry::open(path)?;
        Self::open_entry(entry)
    }
    pub(in crate::host) fn open_entry(entry: crate::paths::private_file::PrivateFileEntry) -> Result<Self> {
        let entry = Arc::new(entry);
        cleanup_stale_creation_ledger_entry(&entry)?;
        let Some(mut ledger): Option<Self> = read_creation_ledger_entry(&entry)? else {
            let mut ledger = Self::default();
            ledger.entry.replace(entry);
            return Ok(ledger);
        };
        ledger.validate()?;
        ledger.entry = Some(entry);
        Ok(ledger)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.schema_version == CREATION_LEDGER_SCHEMA_VERSION,
            "unsupported creation ledger schema {}",
            self.schema_version
        );
        anyhow::ensure!(
            self.bindings.len() <= MAX_CREATION_LEDGER_ENTRIES,
            "creation ledger exceeds entry limit"
        );
        let mut tokens = std::collections::HashSet::new();
        let mut sessions = std::collections::HashSet::new();
        let mut capabilities = std::collections::HashSet::new();
        for (token, result) in self.identities() {
            anyhow::ensure!(
                valid_creation_digest(token),
                "invalid creation token digest"
            );
            anyhow::ensure!(tokens.insert(token), "duplicate creation token digest");
            anyhow::ensure!(!result.terminal_id.is_nil(), "nil creation session identity");
            anyhow::ensure!(
                !result.creation_capability.is_nil(),
                "nil creation capability"
            );
            anyhow::ensure!(
                sessions.insert(result.terminal_id),
                "duplicate session identity"
            );
            anyhow::ensure!(
                capabilities.insert(result.creation_capability),
                "duplicate creation capability"
            );
        }
        for binding in &self.bindings {
            anyhow::ensure!(
                valid_creation_digest(&binding.spec.request_fingerprint),
                "invalid creation request fingerprint"
            );
            anyhow::ensure!(
                valid_creation_cwd(&binding.spec.cwd),
                "creation working directory is not a normalized absolute path"
            );
            anyhow::ensure!(
                (binding.phase == CreationPhase::Reserved) == binding.launch.is_none(),
                CREATION_PHASE_LAUNCH_INCONSISTENT
            );
            let generation_valid = if binding.phase == CreationPhase::Reserved {
                binding.launch_generation == RESERVED_CREATION_LAUNCH_GENERATION
            } else {
                binding.launch_generation >= INITIAL_CREATION_LAUNCH_GENERATION
            };
            anyhow::ensure!(generation_valid, CREATION_LAUNCH_GENERATION_INCONSISTENT);
            if let Some(launch) = &binding.launch {
                anyhow::ensure!(
                    !launch.runtime_epoch.is_nil() && !launch.worker_id.is_nil(),
                    "creation launch identity contains a nil UUID"
                );
                anyhow::ensure!(
                    valid_lower_hex(&launch.auth_token, WORKER_AUTH_TOKEN_HEX_LENGTH),
                    "creation launch auth token is not canonical"
                );
            }
        }
        Ok(())
    }

    pub(crate) fn identities(&self) -> impl Iterator<Item = (&String, CreateTerminalResult)> {
        self.bindings
            .iter()
            .map(|entry| (&entry.creation_token_digest, entry.result))
    }

    pub(crate) fn existing(
        &self,
        token: Uuid,
        request_fingerprint: &str,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<Option<CreationBinding>, ErrorBody> {
        self.ensure_healthy()?;
        let digest = creation_token_digest(token);
        if receipts
            .lookup_token_digest(&digest)
            .map_err(internal)?
            .is_some()
        {
            return Err(ErrorBody::conflict("creation token has been retired"));
        }
        let Some(binding) = self
            .bindings
            .iter()
            .find(|entry| entry.creation_token_digest == digest)
        else {
            return Ok(None);
        };
        if binding.spec.request_fingerprint != request_fingerprint {
            return Err(ErrorBody::conflict(
                "creation token is already bound to a different request",
            ));
        }
        if binding.phase == CreationPhase::Terminated {
            return Err(ErrorBody::conflict("creation cleanup has already started"));
        }
        Ok(Some(binding.clone()))
    }

    pub(crate) fn install(&mut self, path: &Path, candidate: Self) -> std::result::Result<(), ErrorBody> {
        self.install_with(path, candidate, None, |_| Ok(()))
    }

    pub(crate) fn install_before(
        &mut self,
        path: &Path,
        candidate: Self,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        require_creation_budget(deadline)?;
        self.install_with(path, candidate, Some(deadline), |_| Ok(()))
    }

    pub(crate) fn install_with<F>(
        &mut self,
        path: &Path,
        candidate: Self,
        deadline: Option<HostOperationDeadline>,
        mut hook: F,
    ) -> std::result::Result<(), ErrorBody>
    where
        F: FnMut(CreationLedgerWriteStage) -> Result<()>,
    {
        self.ensure_healthy()?;
        candidate.validate().map_err(internal)?;
        let encoded = serde_json::to_vec_pretty(&candidate).map_err(|error| {
            ErrorBody::invalid(format!("creation request cannot be persisted: {error}"))
        })?;
        if encoded.len() as u64 > MAX_CREATION_LEDGER_BYTES {
            return Err(ErrorBody::conflict("creation ledger byte capacity reached"));
        }
        let fence = || match deadline {
            Some(deadline) => require_creation_commit_budget(deadline),
            None => Ok(()),
        };
        let result = match &self.entry {
            Some(entry) => {
                write_encoded_creation_ledger_entry_with_fence(entry, &encoded, &mut hook, fence)
            }
            None => {
                let entry = crate::paths::private_file::PrivateFileEntry::ensure(path)
                    .map_err(CreationLedgerWriteFailure::Unchanged);
                entry.and_then(|entry| {
                    write_encoded_creation_ledger_entry_with_fence(
                        &entry, &encoded, &mut hook, fence,
                    )
                })
            }
        };
        if let Err(failure) = result {
            if failure.is_publication_deadline_expired() {
                return Err(creation_launch_uncertain());
            }
            let uncertain = matches!(&failure, CreationLedgerWriteFailure::Uncertain(_));
            if uncertain {
                self.poisoned = true;
            }
            return Err(internal(failure.into_error()));
        }
        *self = candidate;
        Ok(())
    }
}
