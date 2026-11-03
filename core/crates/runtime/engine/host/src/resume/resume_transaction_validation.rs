use super::*;

impl ResumeTransaction {
    pub(crate) fn new(
        previous_runtime_epoch: Uuid,
        intent: ResumeIntent,
        target: ResumeTarget,
        launch: &WorkerLaunchSpec,
    ) -> Result<Self> {
        let mut transaction = Self {
            schema_version: crate::constants::RESUME_TRANSACTION_SCHEMA_VERSION,
            terminal_id: launch.terminal_id,
            previous_runtime_epoch,
            planned_runtime_epoch: launch.runtime_epoch,
            worker_id: launch.worker_id,
            intent,
            target,
            launch: launch.clone(),
            launch_digest: resume_launch_digest(launch)?,
            transaction_digest: String::new(),
            provider_completion: None,
            created_at_ms: now_ms(),
        };
        transaction.transaction_digest = resume_transaction_digest(&transaction)?;
        transaction.validate_for(launch.terminal_id)?;
        Ok(transaction)
    }

    pub(crate) fn validate_for(&self, terminal_id: Uuid) -> Result<()> {
        anyhow::ensure!(
            self.schema_version == crate::constants::RESUME_TRANSACTION_SCHEMA_VERSION,
            RESUME_TRANSACTION_INVALID
        );
        anyhow::ensure!(self.terminal_id == terminal_id, RESUME_TRANSACTION_INVALID);
        anyhow::ensure!(
            !self.terminal_id.is_nil()
                && !self.previous_runtime_epoch.is_nil()
                && !self.planned_runtime_epoch.is_nil()
                && !self.worker_id.is_nil(),
            RESUME_TRANSACTION_INVALID
        );
        anyhow::ensure!(
            self.previous_runtime_epoch != self.planned_runtime_epoch
                && self.planned_runtime_epoch != self.worker_id,
            RESUME_TRANSACTION_INVALID
        );
        anyhow::ensure!(self.created_at_ms > 0, RESUME_TRANSACTION_INVALID);
        anyhow::ensure!(
            valid_lower_hex(&self.launch_digest, SHA256_HEX_LENGTH),
            RESUME_LAUNCH_DIGEST_INVALID
        );
        anyhow::ensure!(
            valid_lower_hex(&self.transaction_digest, SHA256_HEX_LENGTH)
                && resume_transaction_digest(self)? == self.transaction_digest,
            RESUME_TRANSACTION_DIGEST_INVALID
        );
        anyhow::ensure!(
            self.launch.schema_version == WORKER_LAUNCH_SCHEMA_VERSION
                && self.launch.controller_generation != 0
                && valid_lower_hex(&self.launch.auth_token, WORKER_AUTH_TOKEN_HEX_LENGTH),
            RESUME_TRANSACTION_INVALID
        );
        self.validate_launch(&self.launch)?;
        self.target.validate_for(self.terminal_id, self.intent)?;
        self.validate_provider_completion()?;
        anyhow::ensure!(
            !matches!(
                &self.target,
                ResumeTarget::Shell {
                    binding: ResumeBindingDisposition::Clear { .. }
                }
            ) || self.intent == ResumeIntent::Explicit,
            RESUME_TRANSACTION_INVALID
        );
        Ok(())
    }

    pub(crate) fn validate_launch(&self, launch: &WorkerLaunchSpec) -> Result<()> {
        anyhow::ensure!(
            launch.terminal_id == self.terminal_id
                && launch.runtime_epoch == self.planned_runtime_epoch
                && launch.worker_id == self.worker_id
                && launch == &self.launch,
            RESUME_EVIDENCE_MISMATCH
        );
        anyhow::ensure!(
            resume_launch_digest(launch)? == self.launch_digest,
            RESUME_EVIDENCE_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_provider_completion(&self) -> Result<()> {
        match (&self.target, &self.provider_completion) {
            (ResumeTarget::Shell { .. }, None) | (ResumeTarget::Provider { .. }, None) => Ok(()),
            (ResumeTarget::Provider { .. }, Some(authority)) => authority.validate_for(self),
            (ResumeTarget::Shell { .. }, Some(_)) => {
                anyhow::bail!(RESUME_PROVIDER_AUTHORITY_INVALID)
            }
        }
    }
}
