use super::*;

impl Host {
    pub(crate) async fn retire_previous_runtime(
        &self,
        manifest: &mut TerminalManifest,
    ) -> std::result::Result<(), SessionResumeFailure> {
        let Ok(record) = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
        else {
            return Ok(());
        };
        if self
            .validate_previous_resume_runtime_core(manifest, &record)
            .is_err()
            || validate_runtime_bootstrap(record.bootstrap_delivery).is_err()
        {
            return Ok(());
        }
        if process::boot_identity_relation(&record.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different
        {
            return Ok(());
        }
        if self.secondary_custody.retired_for_settlement(
            &crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
        ) || !process_identity_is_verified_live(record.worker_pid, record.worker_start_time)
        {
            return self
                .settle_disconnected_previous_runtime(manifest, &record)
                .await;
        }
        if let Ok(status) = self.worker_status(&record).await {
            match status.status {
                TerminalRuntimeStatus::Starting | TerminalRuntimeStatus::Running => {
                    return Err(ErrorBody::conflict(RESUME_SESSION_RUNNING).into());
                }
                TerminalRuntimeStatus::Finalizing | TerminalRuntimeStatus::OutputBlocked => {
                    return Err(ErrorBody::conflict(RESUME_FINALIZATION_PENDING).into());
                }
                TerminalRuntimeStatus::Exited
                | TerminalRuntimeStatus::Lost
                | TerminalRuntimeStatus::Unreachable => {}
            }
        }
        if let Err(error) = self.retire_worker_classified(&record).await {
            let settled = error.may_settle_after_disconnect()
                && self.runtime_is_settled(&record, PROCESS_EXIT_TIMEOUT).await;
            if !settled {
                self.persist_recovery_state(manifest, RecoveryState::Uncertain)
                    .await;
                return Err(error.into_error_body().into());
            }
        }
        Ok(())
    }

    pub(crate) async fn settle_disconnected_previous_runtime(
        &self,
        manifest: &mut TerminalManifest,
        record: &RuntimeRecord,
    ) -> std::result::Result<(), SessionResumeFailure> {
        let runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(record);
        let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
        let retirement = loop {
            if self
                .secondary_custody
                .released_for_terminal_worker_shutdown(&runtime)
                && !process_identity_is_gone(record.worker_pid, record.worker_start_time)
            {
                break match self.retire_released_worker_classified(record).await {
                    Ok(()) => WorkerRetirementObservation::Completed,
                    Err(error) if error.may_settle_after_disconnect() => error.observation(),
                    Err(error) => {
                        self.persist_recovery_state(manifest, RecoveryState::Uncertain)
                            .await;
                        return Err(error.into_error_body().into());
                    }
                };
            }
            if self.runtime_is_settled(record, Duration::ZERO).await {
                return Ok(());
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                self.persist_unsettled_previous_runtime_state(manifest, record)
                    .await;
                return Err(self.unresolved_previous_runtime(record).into());
            }
            tokio::time::sleep_until(deadline.min(now + PROCESS_EXIT_POLL_INTERVAL)).await;
        };
        if self
            .runtime_is_settled(
                record,
                crate::session_timing::WORKER_SHUTDOWN_SETTLEMENT_TIMEOUT,
            )
            .await
        {
            return Ok(());
        }
        self.persist_unsettled_previous_runtime_state(manifest, record)
            .await;
        Err(self
            .unresolved_previous_runtime_after_retirement(record, retirement)
            .into())
    }

    pub(crate) async fn persist_unsettled_previous_runtime_state(
        &self,
        manifest: &mut TerminalManifest,
        record: &RuntimeRecord,
    ) {
        // Reconciliation publishes the crash exit after this request releases its session lock.
        if matches!(
            self.automatic_restore_decision_for_runtime(manifest, record),
            AutomaticRestore::Eligible | AutomaticRestore::AwaitingRuntimeSettlement
        ) {
            return;
        }
        self.persist_recovery_state(manifest, RecoveryState::Uncertain)
            .await;
    }

    pub(crate) fn unresolved_previous_runtime(&self, record: &RuntimeRecord) -> ErrorBody {
        self.unresolved_previous_runtime_after_retirement(
            record,
            WorkerRetirementObservation::NotRequested,
        )
    }

    pub(crate) fn unresolved_previous_runtime_after_retirement(
        &self,
        record: &RuntimeRecord,
        retirement: WorkerRetirementObservation,
    ) -> ErrorBody {
        let worker =
            process::process_identity_relation(record.worker_pid, record.worker_start_time);
        let child = process::process_identity_relation(record.child_pid, record.child_start_time);
        let custody = self.secondary_custody.diagnostic_state(
            &crate::pty_custody::CustodyRuntimeIdentity::from_record(record),
        );
        worker_unreachable(format!(
            "{RESUME_PREVIOUS_RUNTIME_UNRESOLVED}: worker={worker:?}, child={child:?}, custody={custody}, retirement={retirement:?}"
        ))
    }
}
