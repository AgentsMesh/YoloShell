use super::*;

pub(crate) const RESUME_ARCHIVED_CONFLICT: &str =
    "archived sessions cannot be resumed; create a new terminal";

impl Host {
    pub(crate) async fn resume_session_locked(
        &self,
        terminal_id: Uuid,
        selection: ResumeSelection,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        self.resume_session_locked_with_product_selection(terminal_id, selection, None)
            .await
    }

    pub(crate) async fn resume_session_locked_with_product_selection(
        &self,
        terminal_id: Uuid,
        selection: ResumeSelection,
        product_selection: Option<TerminalLifecycleSelection>,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        let result = self
            .resume_session_locked_inner(terminal_id, selection, product_selection)
            .await;
        if result.is_err() {
            self.persist_resume_failure_catalog(terminal_id).await;
        }
        result
    }

    pub(crate) async fn resume_session_locked_inner(
        &self,
        terminal_id: Uuid,
        selection: ResumeSelection,
        product_selection: Option<TerminalLifecycleSelection>,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        // The catalog is a derived view. A failed catalog publication can
        // leave it stale or missing after a durable archive or recovery
        // write, so resume starts from the durable manifest.
        let Some(mut manifest) = self
            .load_durable_manifest_optional(terminal_id)
            .map_err(internal)?
        else {
            return Err(ErrorBody::not_found(SESSION_NOT_FOUND).into());
        };
        if manifest.archived {
            // Archiving ends the terminal's resume lifecycle. Allowing a
            // resume here would spawn a Worker that every archived-session
            // lifecycle RPC deliberately ignores.
            return Err(ErrorBody::conflict(RESUME_ARCHIVED_CONFLICT).into());
        }
        match self.load_resume_transaction(terminal_id) {
            Ok(Some(transaction)) if transaction.accepts(selection) => {
                if let Some(rearmed) = self
                    .rearm_dead_resume_terminal(&mut manifest, &transaction)
                    .await?
                {
                    return self.continue_resume_transaction(manifest, rearmed).await;
                }
                self.authorize_parked_resume_retry(&transaction, selection)?;
                return self
                    .continue_resume_transaction(manifest, transaction)
                    .await;
            }
            Ok(Some(_)) => {
                return Err(ErrorBody::conflict(RESUME_TRANSACTION_CONFLICT).into());
            }
            Err(error) => return Err(self.resume_transaction_failure(&mut manifest, error)),
            Ok(None) if manifest.recovery_state == RecoveryState::Restoring => {
                return Err(
                    self.resume_transaction_failure(&mut manifest, RESUME_TRANSACTION_MISSING)
                );
            }
            Ok(None) => {}
        }
        self.begin_resume_transaction(manifest, selection, product_selection.as_ref())
            .await
    }

    pub(crate) async fn begin_resume_transaction(
        &self,
        mut manifest: TerminalManifest,
        selection: ResumeSelection,
        product_selection: Option<&TerminalLifecycleSelection>,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        if manifest.archived {
            return Err(ErrorBody::conflict(RESUME_ARCHIVED_CONFLICT).into());
        }
        let lifecycle_lock = self
            .acquire_resume_lifecycle_lock(manifest.terminal_id, product_selection)
            .await?;
        if selection.intent() == ResumeIntent::Automatic {
            self.ensure_automatic_worker_crash_exit(&manifest).await?;
            #[cfg(test)]
            self.run_after_automatic_crash_exit_probe();
        }
        #[cfg(target_os = "linux")]
        if selection.intent() == ResumeIntent::Explicit {
            self.prepare_linux_crash_exit(&mut manifest).await?;
        }
        let manifest_digest =
            crate::model::TerminalManifestDigest::capture(&manifest).map_err(internal)?;
        drop(lifecycle_lock);
        self.retire_previous_runtime(&mut manifest).await?;
        let lifecycle_lock = self
            .acquire_resume_lifecycle_lock(manifest.terminal_id, product_selection)
            .await?;
        let current_manifest = self
            .load_durable_manifest(manifest.terminal_id)
            .map_err(internal)?;
        if crate::model::TerminalManifestDigest::capture(&current_manifest).map_err(internal)?
            != manifest_digest
        {
            return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED).into());
        }
        let worker_socket = self
            .paths
            .worker_socket(manifest.terminal_id)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        if socket_may_be_serving(&worker_socket).await {
            self.persist_recovery_state(&mut manifest, RecoveryState::Uncertain)
                .await;
            return Err(ErrorBody::conflict(RESUME_SOCKET_ACTIVE).into());
        }
        let previous = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.validate_previous_resume_runtime_core(&manifest, &previous)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let intent = selection.intent();
        if intent == ResumeIntent::Automatic
            && self.automatic_restore_decision_for_runtime(&manifest, &previous)
                != AutomaticRestore::Eligible
        {
            return Err(SessionResumeFailure::AutomaticBlocked(
                self.automatic_resume_block(&manifest, &previous),
            ));
        }
        #[cfg(target_os = "linux")]
        self.settle_linux_recreate_runtime(&previous).await?;
        self.validate_previous_resume_runtime(&manifest, &previous)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let binding = self.binding_for_resume(&mut manifest, selection).await?;
        let target = ResumeTarget::from_binding(&binding);
        let launch = self
            .prepare_resume_launch(&manifest, &binding, intent, previous.runtime_epoch)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        manifest.command = launch.command.clone();
        manifest.args = launch.args.clone();
        let transaction = ResumeTransaction::new(previous.runtime_epoch, intent, target, &launch)
            .map_err(internal)?;
        if let ResumeBinding::Provider(binding) = &binding {
            self.authorize_native_resume(&manifest, binding)
                .map_err(internal)?;
        }
        self.publish_resume_transaction(&transaction)
            .map_err(internal)?;
        self.publish_worker_launch(&launch).map_err(internal)?;
        manifest.recovery_state = RecoveryState::Restoring;
        manifest.updated_at_ms = now_ms();
        self.persist_resume_manifest(&manifest).await?;
        self.stage_resume_binding_disposition_after_lifecycle_lock(&transaction)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        drop(lifecycle_lock);
        self.continue_resume_transaction(manifest, transaction)
            .await
    }
}
