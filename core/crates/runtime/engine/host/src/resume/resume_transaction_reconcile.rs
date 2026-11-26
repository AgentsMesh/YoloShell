use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResumeRetryDisposition {
    ExplicitAllowed,
    EvidenceRequired,
}

impl Host {
    pub(crate) fn resume_transaction_needs_reconcile(&self, manifest: &TerminalManifest) -> bool {
        match self.load_resume_transaction(manifest.terminal_id) {
            Ok(Some(transaction)) => {
                match self.dead_resume_terminal_evidence(manifest, &transaction) {
                    Ok(Some(_)) | Err(_) => true,
                    Ok(None) => match self.actionable_planned_runtime_exists(manifest, &transaction) {
                        Ok(true) => {
                            lock_recover(&self.parked_resume_retries)
                                .remove(&resume_retry_key(&transaction));
                            true
                        }
                        Err(_) => true,
                        Ok(false) => !lock_recover(&self.parked_resume_retries)
                            .contains_key(&resume_retry_key(&transaction)),
                    },
                }
            }
            Err(_) => true,
            Ok(None) => manifest.recovery_state == RecoveryState::Restoring,
        }
    }

    pub(crate) fn park_resume_retry(
        &self,
        terminal_id: Uuid,
        disposition: ResumeRetryDisposition,
    ) -> Result<()> {
        let transaction = self
            .load_resume_transaction(terminal_id)?
            .context(RESUME_TRANSACTION_MISSING)?;
        let manifest = self.load_durable_manifest(terminal_id)?;
        if !self.actionable_planned_runtime_exists(&manifest, &transaction)? {
            lock_recover(&self.parked_resume_retries)
                .insert(resume_retry_key(&transaction), disposition);
        }
        Ok(())
    }

    pub(crate) fn actionable_planned_runtime_exists(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> Result<bool> {
        self.read_optional_resume_json::<RuntimeRecord>(
            &self.paths.runtime_record(manifest.terminal_id),
        )
        .map(|runtime| runtime.is_some_and(|runtime| self.planned_runtime_is_actionable(
            manifest,
            transaction,
            &runtime,
        )))
    }

    pub(crate) fn planned_runtime_is_actionable(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> bool {
        if self
            .validate_planned_runtime(manifest, transaction, runtime)
            .is_err()
        {
            return false;
        }
        match runtime.bootstrap_delivery {
            BootstrapDelivery::Pending => pending_bootstrap_identities_live(runtime),
            BootstrapDelivery::NotRequired | BootstrapDelivery::Delivered => true,
            BootstrapDelivery::Survivor
            | BootstrapDelivery::Failed
            | BootstrapDelivery::Unknown => false,
        }
    }

    pub(crate) fn park_resume_survivor(&self, transaction: &ResumeTransaction) {
        lock_recover(&self.parked_resume_retries).insert(
            resume_retry_key(transaction),
            ResumeRetryDisposition::EvidenceRequired,
        );
    }

    pub(crate) fn authorize_parked_resume_retry(
        &self,
        transaction: &ResumeTransaction,
        selection: ResumeSelection,
    ) -> std::result::Result<(), SessionResumeFailure> {
        let key = resume_retry_key(transaction);
        let mut parked = lock_recover(&self.parked_resume_retries);
        match parked.get(&key) {
            None => Ok(()),
            Some(ResumeRetryDisposition::ExplicitAllowed)
                if selection.intent() == ResumeIntent::Explicit =>
            {
                parked.remove(&key);
                Ok(())
            }
            Some(_) => Err(ErrorBody::conflict(RESUME_TRANSACTION_RETRY_PARKED).into()),
        }
    }

    pub(crate) fn forget_parked_resume_retry(&self, terminal_id: Uuid) {
        lock_recover(&self.parked_resume_retries)
            .retain(|(candidate, _), _| *candidate != terminal_id);
    }

    pub(crate) async fn reconcile_resume_transaction(&self, manifest: TerminalManifest) -> ReconcileResult {
        let terminal_id = manifest.terminal_id;
        match self
            .resume_session_locked(
                terminal_id,
                ResumeSelection::Preserve(ResumeIntent::Automatic),
            )
            .await
        {
            Ok(view)
                if matches!(
                    view.manifest.recovery_state,
                    RecoveryState::Resumed | RecoveryState::Recreated
                ) =>
            {
                ReconcileResult {
                    reattached: 1,
                    ..ReconcileResult::default()
                }
            }
            Ok(_) => ReconcileResult {
                unreachable: 1,
                ..ReconcileResult::default()
            },
            Err(error) => {
                tracing::warn!(%terminal_id, ?error, RESUME_RECONCILE_FAILED_LOG);
                if error.is_terminal() {
                    self.retire_terminal_resume_failure(terminal_id).await;
                }
                ReconcileResult {
                    unreachable: 1,
                    ..ReconcileResult::default()
                }
            }
        }
    }

    /// A non-retryable resume failure means automatic reconcile can never
    /// complete this transaction. Retire it and leave the session uncertain so
    /// the scheduler does not poll a live planned runtime forever.
    pub(crate) async fn retire_terminal_resume_failure(&self, terminal_id: Uuid) {
        let Ok(Some(mut manifest)) = self.load_durable_manifest_optional(terminal_id) else {
            return;
        };
        if manifest.archived {
            return;
        }
        {
            let Ok(Some(_lifecycle)) = self
                .paths
                .try_acquire_session_lifecycle_lock(terminal_id)
            else {
                return;
            };
            if let Err(error) = self.remove_resume_transaction(terminal_id) {
                tracing::warn!(%terminal_id, %error, "retire terminal resume transaction");
                return;
            }
            let launch = self.paths.worker_launch(terminal_id);
            if matches!(
                self.namespace.file_is_missing(&launch),
                Ok(false)
            ) && let Err(error) = self.namespace.remove_file(&launch)
            {
                tracing::warn!(%terminal_id, %error, "retire terminal resume launch");
            }
        }
        if !self
            .persist_recovery_state(&mut manifest, RecoveryState::Uncertain)
            .await
        {
            tracing::error!(%terminal_id, "persist terminal resume retirement");
        }
        tracing::warn!(%terminal_id, "retired a terminal resume transaction; session left uncertain");
    }
}

pub(crate) fn resume_retry_key(transaction: &ResumeTransaction) -> (Uuid, String) {
    (
        transaction.terminal_id,
        transaction.transaction_digest.clone(),
    )
}
