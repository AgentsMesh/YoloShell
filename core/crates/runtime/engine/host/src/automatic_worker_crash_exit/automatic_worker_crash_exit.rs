use crate::paths::identity::FileIdentity;
pub(crate) const AUTOMATIC_CRASH_EXIT_RUNTIME_CHANGED: &str =
    "automatic crash exit runtime evidence changed";
pub(crate) const AUTOMATIC_CRASH_EXIT_UNSAFE: &str =
    "automatic crash exit evidence is not safe to synthesize";
pub(crate) const AUTOMATIC_CRASH_EXIT_JOURNAL_MISSING: &str =
    "automatic crash exit journal is missing";
pub(crate) const AUTOMATIC_CRASH_EXIT_NON_CRASH: &str =
    "automatic crash exit cannot replace a non-crash exit record";
pub(crate) const AUTOMATIC_CRASH_EXIT_SOCKET_ACTIVE: &str =
    "automatic crash exit requires a closed worker socket";
pub(crate) const AUTOMATIC_CRASH_EXIT_CUSTODY_MISSING: &str =
    "automatic crash exit requires retired PTY custody";
pub(crate) const AUTOMATIC_CRASH_EXIT_PARENT_MISSING: &str =
    "automatic crash exit record has no retained parent";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ExitEvidence {
    Missing,
    Stale(FileIdentity),
    MatchingWorkerCrash {
        exit: ExitRecord,
        identity: FileIdentity,
    },
}

impl Host {
    pub(crate) async fn ensure_worker_crash_exit_inner(
        &self,
        manifest: &TerminalManifest,
    ) -> Result<()> {
        let (runtime, runtime_identity) = self.read_runtime_with_identity(manifest.terminal_id)?;
        self.validate_previous_resume_runtime_core(manifest, &runtime)?;
        let boot_relation = process::boot_identity_relation(&runtime.boot_id, &self.boot_id);
        if boot_relation == process::BootIdentityRelation::Different {
            return Ok(());
        }
        anyhow::ensure!(
            boot_relation == process::BootIdentityRelation::Same,
            AUTOMATIC_CRASH_EXIT_UNSAFE
        );
        anyhow::ensure!(
            matches!(
                runtime.bootstrap_delivery,
                BootstrapDelivery::NotRequired | BootstrapDelivery::Delivered
            ),
            AUTOMATIC_CRASH_EXIT_UNSAFE
        );
        anyhow::ensure!(
            self.exit_allows_restore(&runtime)
                && self.termination_tombstone_state(&runtime) == TerminationTombstoneState::Missing,
            AUTOMATIC_CRASH_EXIT_UNSAFE
        );
        #[cfg(target_os = "linux")]
        linux_retirement::settle_after_lifecycle_lock(self, &runtime).await?;
        anyhow::ensure!(
            process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time)
                && process_identity_is_gone(runtime.child_pid, runtime.child_start_time),
            AUTOMATIC_CRASH_EXIT_UNSAFE
        );
        let custody = self
            .secondary_custody
            .retirement_evidence(
                self,
                &crate::pty_custody::CustodyRuntimeIdentity::from_record(&runtime),
            )
            .ok_or_else(|| anyhow::anyhow!(AUTOMATIC_CRASH_EXIT_CUSTODY_MISSING))?;
        custody.verify(self)?;
        self.ensure_socket_closed(&runtime.socket_path).await?;
        let worker_socket = self.paths.worker_socket(manifest.terminal_id)?;
        self.ensure_socket_closed(&worker_socket).await?;

        let exit_path = self.paths.exit_record(manifest.terminal_id);
        let exit_evidence = self.read_exit_evidence(&exit_path, runtime.runtime_epoch)?;
        let expected_exit_identity = match &exit_evidence {
            ExitEvidence::MatchingWorkerCrash { .. } => None,
            ExitEvidence::Missing => None,
            ExitEvidence::Stale(identity) => Some(*identity),
        };
        let journal_path = self.paths.journal(manifest.terminal_id);
        anyhow::ensure!(
            !self.namespace.file_is_missing(&journal_path)?,
            AUTOMATIC_CRASH_EXIT_JOURNAL_MISSING
        );
        let journal = crate::journal::Journal::inspect_retained_evidence(
            self.namespace.file(&journal_path)?,
            runtime.runtime_epoch,
        )
        .context(AUTOMATIC_CRASH_EXIT_UNSAFE)?;
        let last_output_sequence = journal.last_sequence();
        self.ensure_runtime_unchanged(manifest.terminal_id, &runtime, runtime_identity)?;
        journal
            .verify_current()
            .context(AUTOMATIC_CRASH_EXIT_UNSAFE)?;
        if let ExitEvidence::MatchingWorkerCrash { exit, identity } = exit_evidence {
            anyhow::ensure!(
                exit.code.is_none()
                    && exit.signal.is_none()
                    && exit.last_output_sequence == last_output_sequence,
                AUTOMATIC_CRASH_EXIT_UNSAFE
            );
            let parent = exit_path
                .parent()
                .context(AUTOMATIC_CRASH_EXIT_PARENT_MISSING)?;
            self.namespace.directory(parent)?.sync()?;
            #[cfg(test)]
            automatic_worker_crash_exit_test_hook::run(manifest.terminal_id)?;
            let (current, current_identity) = self
                .namespace
                .read_json_with_identity::<ExitRecord>(&exit_path)?;
            anyhow::ensure!(
                current == exit && current_identity == identity,
                AUTOMATIC_CRASH_EXIT_UNSAFE
            );
            journal
                .verify_current()
                .context(AUTOMATIC_CRASH_EXIT_UNSAFE)?;
            custody.verify(self)?;
            self.ensure_runtime_unchanged(manifest.terminal_id, &runtime, runtime_identity)?;
            return Ok(());
        }

        let exit = ExitRecord {
            runtime_epoch: runtime.runtime_epoch,
            code: None,
            signal: None,
            reason: ExitReason::WorkerCrash,
            observed_at_ms: crate::model::now_ms(),
            last_output_sequence,
        };
        let installed_identity = self.namespace.write_json_expected_with_commit_fence(
            &exit_path,
            expected_exit_identity,
            &exit,
            || {
                #[cfg(test)]
                automatic_worker_crash_exit_test_hook::run(manifest.terminal_id)?;
                self.ensure_runtime_unchanged(manifest.terminal_id, &runtime, runtime_identity)?;
                journal
                    .verify_current()
                    .context(AUTOMATIC_CRASH_EXIT_UNSAFE)?;
                custody.verify(self)?;
                Ok(())
            },
        )?;
        self.ensure_runtime_unchanged(manifest.terminal_id, &runtime, runtime_identity)?;
        journal
            .verify_current()
            .context(AUTOMATIC_CRASH_EXIT_UNSAFE)?;
        custody.verify(self)?;
        let (installed, current_identity) = self
            .namespace
            .read_json_with_identity::<ExitRecord>(&exit_path)?;
        anyhow::ensure!(
            installed == exit && current_identity == installed_identity,
            AUTOMATIC_CRASH_EXIT_UNSAFE
        );
        Ok(())
    }
}
