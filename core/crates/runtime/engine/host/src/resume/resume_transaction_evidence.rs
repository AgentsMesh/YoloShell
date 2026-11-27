use super::*;

impl Host {
    pub(crate) fn validate_previous_resume_runtime_core(
        &self,
        manifest: &TerminalManifest,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        anyhow::ensure!(
            runtime.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
                && runtime.terminal_id == manifest.terminal_id
                && !runtime.runtime_epoch.is_nil()
                && runtime.worker_start_time != 0
                && runtime.child_start_time != 0,
            RESUME_EVIDENCE_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_previous_resume_runtime(
        &self,
        manifest: &TerminalManifest,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        self.validate_previous_resume_runtime_core(manifest, runtime)?;
        let previous_boot = process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different;
        let identities_gone =
            process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time)
                && process_identity_is_gone(runtime.child_pid, runtime.child_start_time);
        anyhow::ensure!(
            previous_boot || identities_gone,
            RESUME_PREVIOUS_RUNTIME_UNRESOLVED
        );
        Ok(())
    }

    pub(crate) fn validate_planned_runtime(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        self.validate_planned_runtime_core(manifest, transaction, runtime)?;
        anyhow::ensure!(
            process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
                == process::BootIdentityRelation::Same,
            RESUME_EVIDENCE_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_committed_resume_runtime(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        self.validate_ready_resume_runtime_core(manifest, transaction, runtime)?;
        anyhow::ensure!(
            process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
                != process::BootIdentityRelation::Unknown,
            RESUME_EVIDENCE_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_planned_runtime_core(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        self.validate_planned_runtime_identity_core(manifest, transaction, runtime)?;
        // The live PTY can resize while provider hooks are still settling.
        // Geometry is mutable presentation state, not launch identity.
        anyhow::ensure!(
            normalize_terminal_columns(runtime.cols) == runtime.cols
                && normalize_terminal_rows(runtime.rows) == runtime.rows,
            RESUME_EVIDENCE_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_planned_runtime_identity_core(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        let launch = &transaction.launch;
        self.validate_transaction_launch(manifest, transaction, launch)?;
        anyhow::ensure!(
            runtime.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
                && runtime.terminal_id == transaction.terminal_id
                && runtime.runtime_epoch == transaction.planned_runtime_epoch
                && runtime.worker_id == transaction.worker_id
                && runtime.socket_path == launch.socket_path
                && runtime.auth_token == launch.auth_token
                && runtime.command == launch.command
                && runtime.args == launch.args
                && runtime.cwd == launch.cwd
                && runtime.started_at_ms > 0,
            RESUME_EVIDENCE_MISMATCH
        );
        anyhow::ensure!(
            runtime.controller_generation >= launch.controller_generation
                && runtime.controller_generation <= self.controller_generation
                && runtime.worker_pid != 0
                && runtime.child_pid != 0
                && runtime.worker_pid != runtime.child_pid
                && runtime.worker_start_time != 0
                && runtime.child_start_time != 0,
            RESUME_EVIDENCE_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_resume_delivery(
        &self,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        let expected = match &transaction.target {
            ResumeTarget::Shell { .. } => BootstrapDelivery::NotRequired,
            ResumeTarget::Provider { .. } => BootstrapDelivery::Delivered,
        };
        anyhow::ensure!(
            runtime.bootstrap_delivery == expected,
            RESUME_EVIDENCE_MISMATCH
        );
        Ok(())
    }
}
