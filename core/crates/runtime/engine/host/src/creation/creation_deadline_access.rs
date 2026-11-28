use super::*;

impl Host {
    pub(in crate::host) fn write_creation_json_before<T: Serialize>(
        &self,
        path: &Path,
        value: &T,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        self.write_creation_json_raw_before(path, value, deadline)
            .map_err(creation_publication_error)
    }

    pub(in crate::host) fn write_creation_json_raw_before<T: Serialize>(
        &self,
        path: &Path,
        value: &T,
        deadline: HostOperationDeadline,
    ) -> Result<()> {
        require_creation_publication_budget(deadline)?;
        self.namespace
            .write_json_with_commit_fence(path, value, || require_creation_commit_budget(deadline))
    }

    pub(crate) fn ensure_creation_directory_before(
        &self,
        path: &Path,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        require_creation_budget(deadline)?;
        self.namespace
            .ensure_directory_with_commit_fence(path, || require_creation_commit_budget(deadline))
            .map(|_| ())
            .map_err(creation_publication_error)
    }

    pub(crate) fn ensure_session_namespace_before(
        &self,
        terminal_id: Uuid,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        for path in [
            self.paths.session_dir(terminal_id),
            self.paths.terminal_dir(terminal_id),
            self.paths.events_dir(terminal_id),
            self.paths.logs_dir(terminal_id),
        ] {
            self.ensure_creation_directory_before(&path, deadline)?;
        }
        Ok(())
    }

    pub(crate) async fn lock_creation_capabilities_before(
        &self,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<MutexGuard<'_, CreationLedger>, ErrorBody> {
        loop {
            require_creation_budget(deadline)?;
            if let Some(guard) = self.try_lock_creation_capabilities() {
                return Ok(guard);
            }
            deadline
                .wait_for_poll(CREATION_LEDGER_LOCK_POLL_INTERVAL)
                .await;
        }
    }

    pub(crate) fn try_lock_creation_capabilities(&self) -> Option<MutexGuard<'_, CreationLedger>> {
        match self.creation_capabilities.try_lock() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                tracing::error!(message = CREATION_LEDGER_POISON_RECOVERY_LOG);
                Some(poisoned.into_inner())
            }
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    pub(crate) async fn existing_creation_before(
        &self,
        path: &Path,
        token: Uuid,
        fingerprint: &str,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<Option<CreationBinding>, ErrorBody> {
        let mut ledger = self.lock_creation_capabilities_before(deadline).await?;
        ledger.recover_token_completion_before(path, &self.creation_receipts, token, deadline)?;
        let result = ledger.existing(token, fingerprint, &self.creation_receipts);
        require_creation_budget(deadline)?;
        result
    }

    pub(crate) async fn reserve_creation_before(
        &self,
        path: &Path,
        token: Uuid,
        spec: CreationSpec,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<CreationBinding, ErrorBody> {
        let mut ledger = self.lock_creation_capabilities_before(deadline).await?;
        ledger.recover_token_completion_before(path, &self.creation_receipts, token, deadline)?;
        let existing = ledger.existing(token, &spec.request_fingerprint, &self.creation_receipts);
        require_creation_budget(deadline)?;
        if let Some(binding) = existing? {
            return Ok(binding);
        }
        ledger.reserve_before(path, token, spec, &self.creation_receipts, deadline)
    }

    pub(crate) async fn prepare_creation_launch_attempt_before(
        &self,
        ledger_path: &Path,
        result: CreateTerminalResult,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(CreationBinding, CreationLaunchIdentity), ErrorBody> {
        let mut ledger = self.lock_creation_capabilities_before(deadline).await?;
        ledger.prepare_launch_attempt_before(ledger_path, result, deadline)
    }

    pub(crate) async fn persist_creation_phase_before(
        &self,
        path: &Path,
        result: CreateTerminalResult,
        phase: CreationPhase,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        let mut ledger = self.lock_creation_capabilities_before(deadline).await?;
        ledger.set_active_phase_before(path, result, phase, deadline)
    }
}
