use super::*;

impl Host {
    pub(crate) fn dead_resume_terminal_evidence(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> Result<Option<RuntimeRecord>> {
        transaction.validate_for(manifest.terminal_id)?;
        let Some(runtime) = self.read_optional_resume_json::<RuntimeRecord>(
            &self.paths.runtime_record(manifest.terminal_id),
        )?
        else {
            return Ok(None);
        };
        if runtime.runtime_epoch != transaction.planned_runtime_epoch {
            return Ok(None);
        }
        let previous_boot = process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different;
        match runtime.bootstrap_delivery {
            BootstrapDelivery::Survivor | BootstrapDelivery::Failed => {
                self.validate_planned_runtime_core(manifest, transaction, &runtime)?;
            }
            BootstrapDelivery::NotRequired | BootstrapDelivery::Delivered => {
                self.validate_ready_resume_runtime_core(manifest, transaction, &runtime)?;
                if !previous_boot && !self.ready_runtime_custody_is_settled(&runtime) {
                    return Ok(None);
                }
            }
            BootstrapDelivery::Pending | BootstrapDelivery::Unknown => return Ok(None),
        }
        let identities_gone =
            process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time)
                && process_identity_is_gone(runtime.child_pid, runtime.child_start_time);
        if !previous_boot && !identities_gone {
            return Ok(None);
        }
        let current = self.read_namespace_json::<RuntimeRecord>(
            &self.paths.runtime_record(manifest.terminal_id),
        )?;
        anyhow::ensure!(current == runtime, RESUME_AUTHENTICATED_RUNTIME_MISMATCH);
        Ok(Some(runtime))
    }

    pub(crate) fn ready_runtime_custody_is_settled(&self, runtime: &RuntimeRecord) -> bool {
        self.secondary_custody.retired_for_settlement(
            &crate::pty_custody::CustodyRuntimeIdentity::from_record(runtime),
        )
    }
}
