include!("reconcile_driver.rs");

impl Host {
    fn finish_recovery(&self, ticket: &yolo_host_change_feed::recovery::RecoveryTicket) {
        if self.recovery_queue.settle(ticket) {
            self.watch.feed.touch(Some(ticket.terminal_id));
        }
    }

    #[cfg(test)]
    pub(crate) async fn reconcile_manifest(&self, manifest: TerminalManifest) -> ReconcileResult {
        self.reconcile_ticket(self.recovery_queue.schedule(manifest.terminal_id)).await
    }

    pub(crate) async fn reconcile_ticket(
        &self,
        ticket: yolo_host_change_feed::recovery::RecoveryTicket,
    ) -> ReconcileResult {
        let terminal_id = ticket.terminal_id;
        #[cfg(test)]
        self.panic_reconciliation_if_requested(terminal_id);
        let mut result = ReconcileResult::default();
        let _session_guard = self.session_locks.lock(terminal_id).await;
        #[cfg(feature = "reconcile-test-hooks")]
        reconcile_test_hook::pause_after_session_lock(terminal_id).await;
        if !self.settle_reboot_migration(terminal_id) {
            result.unreachable += RECONCILIATION_FAILURE_COUNT;
            return result;
        }
        let ledger_path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        if let Err(error) = self.recover_session_creation(&ledger_path, terminal_id) {
            result.unreachable += RECONCILIATION_FAILURE_COUNT;
            tracing::error!(terminal_id = %terminal_id, ?error, "creation receipt recovery failed");
            return result;
        }
        let active_creation = match lock_recover(&self.creation_capabilities)
            .active_creation(terminal_id)
        {
            Ok(active) => active.is_some(),
            Err(error) => {
                result.unreachable += RECONCILIATION_FAILURE_COUNT;
                tracing::error!(terminal_id = %terminal_id, ?error, "creation ledger unavailable during reconciliation");
                return result;
            }
        };
        let mut manifest = match self.catalog_get(terminal_id).await {
            Ok(Some(current)) if !current.archived => current,
            Ok(_) => {
                self.finish_recovery(&ticket);
                return result;
            }
            Err(error) => {
                tracing::error!(%terminal_id, %error, "catalog session unavailable for recovery");
                return isolated_reconciliation_failure();
            }
        };
        if !active_creation && self.resume_transaction_needs_reconcile(&manifest) {
            return self.reconcile_resume_transaction(manifest).await;
        }
        let runtime = self.read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(terminal_id)).ok();
        if let Some(record) = runtime.as_ref().filter(|record| dead_pending_record(terminal_id, record)) {
            result.unreachable += RECONCILIATION_FAILURE_COUNT;
            if self.persist_recovery_state(&mut manifest, RecoveryState::Uncertain).await
                && !active_creation && self.recovery_finished(&manifest, record)
            {
                self.finish_recovery(&ticket);
            }
            return result;
        }
        let current = runtime.as_ref().filter(|record| {
            validate_live_worker(record, terminal_id, &self.boot_id).is_ok()
        });
        let restore = if active_creation || current.is_some() {
            AutomaticRestore::BlockedByRuntime
        } else {
            runtime.as_ref().map_or(AutomaticRestore::Ineligible, |record| self.classify_automatic_restore(&manifest, record))
        };
        #[cfg(test)]
        self.pause_after_restore_decision(restore).await;
        // A verified live record goes through the authenticated Worker status
        // check. Recovery of a missing Worker re-reads the record after the decision.
        let live = match current {
            Some(record) => Ok(record.clone()),
            None => self.live_runtime_record(terminal_id).await,
        };
        match live {
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
        if !active_creation && restore == AutomaticRestore::Ineligible
            && runtime.as_ref().is_some_and(|record| self.recovery_finished(&manifest, record)) {
            self.finish_recovery(&ticket);
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
