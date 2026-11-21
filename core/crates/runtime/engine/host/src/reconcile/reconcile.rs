include!("reconcile_driver.rs");

impl Host {
    pub(crate) async fn reconcile_manifest(
        &self,
        mut manifest: TerminalManifest,
    ) -> ReconcileResult {
        #[cfg(test)]
        self.panic_reconciliation_if_requested(manifest.terminal_id);
        let mut result = ReconcileResult::default();
        let _session_guard = self.session_locks.lock(manifest.terminal_id).await;
        #[cfg(feature = "reconcile-test-hooks")]
        reconcile_test_hook::pause_after_session_lock(manifest.terminal_id).await;
        if !self.settle_reboot_migration(manifest.terminal_id) {
            result.unreachable += RECONCILIATION_FAILURE_COUNT;
            return result;
        }
        let ledger_path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        if let Err(error) = self.recover_session_creation(&ledger_path, manifest.terminal_id) {
            result.unreachable += RECONCILIATION_FAILURE_COUNT;
            tracing::error!(terminal_id = %manifest.terminal_id, ?error, "creation receipt recovery failed");
            return result;
        }
        let active_creation = match lock_recover(&self.creation_capabilities)
            .active_creation(manifest.terminal_id)
        {
            Ok(active) => active.is_some(),
            Err(error) => {
                result.unreachable += RECONCILIATION_FAILURE_COUNT;
                tracing::error!(terminal_id = %manifest.terminal_id, ?error, "creation ledger unavailable during reconciliation");
                return result;
            }
        };
        match self.catalog_get(manifest.terminal_id).await {
            Ok(Some(current)) if !current.archived => manifest = current,
            Ok(_) => return result,
            Err(error) => {
                result.unreachable += RECONCILIATION_FAILURE_COUNT;
                tracing::error!(
                    terminal_id = %manifest.terminal_id,
                    %error,
                    "catalog session changed before reconciliation"
                );
                return result;
            }
        }
        if !active_creation && self.resume_transaction_needs_reconcile(&manifest) {
            return self.reconcile_resume_transaction(manifest).await;
        }
        if self.dead_pending_bootstrap(&manifest) {
            result.unreachable += RECONCILIATION_FAILURE_COUNT;
            self.persist_recovery_state(&mut manifest, RecoveryState::Uncertain)
                .await;
            return result;
        }
        let restore = if active_creation {
            AutomaticRestore::BlockedByRuntime
        } else {
            self.automatic_restore_decision(&manifest)
        };
        #[cfg(test)]
        self.pause_after_restore_decision(restore).await;
        match self.live_runtime_record(manifest.terminal_id).await {
            Ok(record) if self.current_child_exit_is_durable(&record) => {
                // The exact child exit is already durable. Do not re-probe the
                // Worker on every reconciliation pass; its process may still be
                // draining the last attachment before exiting.
                self.worker_connections.clear(record.terminal_id).await;
                result.exited += RECONCILIATION_FAILURE_COUNT;
            }
            Ok(record) => match self.worker_status(&record).await {
                Ok(status)
                    if matches!(
                        status.status,
                        TerminalRuntimeStatus::Starting | TerminalRuntimeStatus::Running
                    ) =>
                {
                    result.reattached += RECONCILIATION_FAILURE_COUNT;
                    let survivor = record.bootstrap_delivery == BootstrapDelivery::Survivor;
                    let recovery_state = if survivor {
                        RecoveryState::Uncertain
                    } else {
                        RecoveryState::Reattached
                    };
                    if survivor && manifest.recovery_state != RecoveryState::Uncertain
                        || !survivor
                            && matches!(
                                manifest.recovery_state,
                                RecoveryState::Created
                                    | RecoveryState::Restoring
                                    | RecoveryState::Uncertain
                            )
                    {
                        self.persist_recovery_state(&mut manifest, recovery_state)
                            .await;
                    }
                }
                Ok(status) if status.status == TerminalRuntimeStatus::Exited => {
                    result.exited += RECONCILIATION_FAILURE_COUNT;
                }
                Ok(_) => result.unreachable += RECONCILIATION_FAILURE_COUNT,
                Err(error) => {
                    result.unreachable += RECONCILIATION_FAILURE_COUNT;
                    tracing::warn!(
                        terminal_id = %manifest.terminal_id,
                        %error,
                        "worker status is temporarily unreachable"
                    );
                }
            },
            Err(error)
                if restore == AutomaticRestore::Eligible && recoverable_runtime_loss(&error) =>
            {
                result.unreachable += RECONCILIATION_FAILURE_COUNT;
                match self
                    .resume_session_locked(
                        manifest.terminal_id,
                        ResumeSelection::Preserve(ResumeIntent::Automatic),
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(failure) => {
                        if self
                            .reconcile_resume_failure(manifest.terminal_id, failure)
                            .await
                        {
                            result.needs_recreate += RECONCILIATION_FAILURE_COUNT;
                        }
                    }
                }
            }
            Err(error)
                if restore == AutomaticRestore::AwaitingRuntimeSettlement
                    && recoverable_runtime_loss(&error) =>
            {
                result.unreachable += RECONCILIATION_FAILURE_COUNT;
                tracing::debug!(
                    terminal_id = %manifest.terminal_id,
                    "automatic recovery is waiting for the previous child runtime to settle"
                );
            }
            Err(error)
                if restore == AutomaticRestore::BlockedByRuntime
                    && recoverable_runtime_loss(&error) =>
            {
                result.unreachable += RECONCILIATION_FAILURE_COUNT;
                self.persist_recovery_state(&mut manifest, RecoveryState::Uncertain)
                    .await;
            }
            Err(error) if error.code == crate::protocol::error_code::HOST_REBOOTED => {
                result.unreachable += RECONCILIATION_FAILURE_COUNT;
                result.needs_recreate += RECONCILIATION_FAILURE_COUNT;
                self.persist_recovery_state(&mut manifest, RecoveryState::NeedsRecreate)
                    .await;
            }
            Err(_) => result.unreachable += RECONCILIATION_FAILURE_COUNT,
        }
        result
    }
}

#[cfg(any(test, feature = "reconcile-test-hooks"))]
#[doc(hidden)]
pub mod reconcile_test_hook {
    #[cfg(any(not(test), feature = "yolo-unit-tests"))]
    include!("../../test_support/unit/reconcile_test_hook.rs");
}

include!("reconcile_catalog.rs");
include!("reconcile_helpers.rs");
include!("reconcile_resume_failure.rs");
