impl Host {
    pub(crate) async fn ensure_automatic_worker_crash_exit(
        &self,
        manifest: &TerminalManifest,
    ) -> std::result::Result<(), SessionResumeFailure> {
        let expected_runtime = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .ok();
        self.ensure_automatic_worker_crash_exit_inner(manifest)
            .await
            .map_err(|error| {
                #[cfg(target_os = "linux")]
                if error.is::<linux_retirement::Pending>() {
                    return worker_unreachable(error.to_string()).into();
                }
                tracing::warn!(
                    terminal_id = %manifest.terminal_id,
                    %error,
                    "automatic worker crash exit synthesis failed closed"
                );
                let block = expected_runtime
                    .as_ref()
                    .map_or(AutomaticResumeBlock::RecoveryEvidenceChanged, |runtime| {
                        self.automatic_resume_block(manifest, runtime)
                    });
                SessionResumeFailure::AutomaticBlocked(block)
            })
    }

    pub(crate) async fn ensure_automatic_worker_crash_exit_inner(
        &self,
        manifest: &TerminalManifest,
    ) -> Result<()> {
        let (runtime, _) = self.read_runtime_with_identity(manifest.terminal_id)?;
        if process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
            != process::BootIdentityRelation::Different
        {
            anyhow::ensure!(
                self.automatic_restore_decision_for_runtime(manifest, &runtime)
                    == AutomaticRestore::Eligible,
                AUTOMATIC_CRASH_EXIT_UNSAFE
            );
        }
        self.ensure_worker_crash_exit_inner(manifest).await
    }

    #[cfg(target_os = "linux")]
    pub(crate) async fn prepare_linux_crash_exit(
        &self,
        manifest: &mut TerminalManifest,
    ) -> std::result::Result<(), SessionResumeFailure> {
        let Ok((runtime, _)) = self.read_runtime_with_identity(manifest.terminal_id) else {
            // Missing/invalid runtime and serving sockets belong to the existing
            // resume transaction admission, which retains its conflict contract.
            return Ok(());
        };
        if process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
            != process::BootIdentityRelation::Same
            || self
                .validate_previous_resume_runtime_core(manifest, &runtime)
                .is_err()
            || !runtime.bootstrap_delivery.is_ready()
            || !process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time)
            || !self.exit_allows_restore(&runtime)
            || self.termination_tombstone_state(&runtime) != TerminationTombstoneState::Missing
        {
            return Ok(());
        }
        match self.ensure_worker_crash_exit_inner(manifest).await {
            Ok(()) => Ok(()),
            Err(error) => {
                if !error.is::<linux_retirement::Pending>() {
                    self.persist_recovery_state(manifest, RecoveryState::Uncertain)
                        .await;
                }
                Err(worker_unreachable(error.to_string()).into())
            }
        }
    }
}
