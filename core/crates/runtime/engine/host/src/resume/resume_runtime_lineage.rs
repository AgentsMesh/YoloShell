use super::*;

impl Host {
    pub(crate) fn validate_initial_completion_runtime(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        if let Some(authority) = transaction.provider_completion.as_ref() {
            authority.validate_for(transaction)?;
            authority.validate_runtime(runtime)?;
            self.validate_receipted_runtime(manifest, transaction, runtime)
        } else {
            self.validate_planned_runtime(manifest, transaction, runtime)?;
            self.validate_resume_delivery(transaction, runtime)?;
            anyhow::ensure!(
                resume_runtime_is_live(runtime),
                RESUME_AUTHENTICATED_RUNTIME_MISMATCH
            );
            Ok(())
        }
    }

    pub(crate) fn validate_resume_runtime_update(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        previous: &RuntimeRecord,
        candidate: &RuntimeRecord,
    ) -> Result<()> {
        candidate
            .validate_update_from(previous)
            .map_err(|error| anyhow::anyhow!("{RESUME_AUTHENTICATED_RUNTIME_MISMATCH}: {error}"))?;
        self.validate_planned_runtime_identity_core(manifest, transaction, candidate)?;
        self.validate_resume_delivery(transaction, candidate)?;
        anyhow::ensure!(
            process::boot_identity_relation(&candidate.boot_id, &self.boot_id)
                == process::BootIdentityRelation::Same
                && candidate.controller_generation == self.controller_generation
                && process_identity_is_verified_live(
                    candidate.worker_pid,
                    candidate.worker_start_time
                )
                && process_identity_is_verified_live(
                    candidate.child_pid,
                    candidate.child_start_time
                ),
            RESUME_AUTHENTICATED_RUNTIME_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_receipted_runtime(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        self.validate_ready_resume_runtime_core(manifest, transaction, runtime)?;
        anyhow::ensure!(
            process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
                == process::BootIdentityRelation::Same
                && resume_runtime_is_live(runtime),
            RESUME_AUTHENTICATED_RUNTIME_MISMATCH
        );
        Ok(())
    }

    pub(crate) fn validate_ready_resume_runtime_core(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        self.validate_planned_runtime_identity_core(manifest, transaction, runtime)?;
        self.validate_resume_delivery(transaction, runtime)?;
        anyhow::ensure!(
            normalize_terminal_columns(runtime.cols) == runtime.cols
                && normalize_terminal_rows(runtime.rows) == runtime.rows,
            RESUME_AUTHENTICATED_RUNTIME_MISMATCH
        );
        Ok(())
    }
}

pub(crate) fn resume_runtime_is_live(runtime: &RuntimeRecord) -> bool {
    process_identity_is_verified_live(runtime.worker_pid, runtime.worker_start_time)
        && process_identity_is_verified_live(runtime.child_pid, runtime.child_start_time)
}
