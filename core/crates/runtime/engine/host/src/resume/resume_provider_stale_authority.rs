use super::*;

impl Host {
    /// When a planned provider resume has timed out and the terminal's native
    /// authority belongs to another runtime, the new Worker can never publish
    /// the expected evidence. Clear the stale authority and binding, retire the
    /// resume transaction, and leave the session uncertain. The live PTY and
    /// its shell process are untouched.
    pub(crate) fn settle_pending_provider_resume(
        &self,
        manifest: &mut TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> Result<bool> {
        {
            let Some(_lifecycle) = self
                .paths
                .try_acquire_session_lifecycle_lock(manifest.terminal_id)?
            else {
                return Ok(false);
            };
            let current = self.read_namespace_json::<RuntimeRecord>(
                &self.paths.runtime_record(manifest.terminal_id),
            )?;
            self.validate_planned_runtime_identity_core(manifest, transaction, &current)?;
            let identity = crate::agent::authority::AuthorityRuntimeIdentity {
                runtime_epoch: current.runtime_epoch,
                worker: crate::agent::authority::AuthorityProcessIdentity {
                    pid: current.worker_pid,
                    start_time: current.worker_start_time,
                },
            };
            let store = self.agent_binding_store(current.terminal_id);
            if !store.has_foreign_native_authority_in_namespace(&self.namespace, identity)? {
                return Ok(false);
            }
            let guard = yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_after_lifecycle_lock(
                &self.paths,
                &self.namespace,
                manifest.terminal_id,
            )?;
            if !store.has_foreign_native_authority_in_namespace(&self.namespace, identity)? {
                return Ok(false);
            }
            store.revoke_native_authorities_with_guard(&guard, || {
                guard.require_no_pending_intent()
            })?;
            self.namespace.remove_file(&store.path())?;
        }
        self.remove_resume_transaction(manifest.terminal_id)?;
        let launch = self.paths.worker_launch(manifest.terminal_id);
        if !self.namespace.file_is_missing(&launch)? {
            self.namespace.remove_file(&launch)?;
        }
        self.persist_recovery_state_durable(manifest, RecoveryState::Uncertain);
        tracing::warn!(
            terminal_id = %manifest.terminal_id,
            "provider resume authority belongs to a previous runtime; cleared it and retired automatic recovery"
        );
        Ok(true)
    }
}
