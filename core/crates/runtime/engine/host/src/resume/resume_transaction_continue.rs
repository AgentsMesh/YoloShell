use super::*;

impl Host {
    pub(crate) async fn continue_resume_transaction(
        &self,
        mut manifest: TerminalManifest,
        transaction: ResumeTransaction,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        transaction
            .validate_for(manifest.terminal_id)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let durable = self
            .load_durable_manifest(manifest.terminal_id)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        manifest = durable;
        if manifest.archived {
            return Err(ErrorBody::conflict(RESUME_ARCHIVED_CONFLICT).into());
        }
        if manifest.recovery_state == transaction.target.completed_state() {
            return self.finish_committed_resume(manifest, &transaction).await;
        }
        if !matches!(
            manifest.recovery_state,
            RecoveryState::Created
                | RecoveryState::Reattached
                | RecoveryState::Restoring
                | RecoveryState::Uncertain
                | RecoveryState::NeedsRecreate
        ) {
            return Err(self.resume_transaction_failure(&mut manifest, RESUME_MANIFEST_MISMATCH));
        }
        self.validate_transaction_launch(&manifest, &transaction, &transaction.launch)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let launch = self
            .read_optional_resume_json::<WorkerLaunchSpec>(
                &self.paths.worker_launch(manifest.terminal_id),
            )
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        if let Some(launch) = &launch {
            self.validate_transaction_launch(&manifest, &transaction, launch)
                .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        }
        let runtime = self
            .read_optional_resume_json::<RuntimeRecord>(
                &self.paths.runtime_record(manifest.terminal_id),
            )
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?
            .ok_or_else(|| {
                self.resume_transaction_failure(&mut manifest, RESUME_EVIDENCE_MISMATCH)
            })?;
        if runtime.runtime_epoch == transaction.previous_runtime_epoch {
            self.validate_previous_resume_runtime_core(&manifest, &runtime)
                .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
            #[cfg(target_os = "linux")]
            self.settle_linux_recreate_runtime(&runtime).await?;
            self.validate_previous_resume_runtime(&manifest, &runtime)
                .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
            let launch = self
                .stage_resume_binding_disposition_with_replay_launch(&transaction)
                .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
            self.ensure_restoring_manifest(&mut manifest).await?;
            return self
                .replay_resume_launch(manifest, transaction, launch)
                .await;
        }
        if runtime.runtime_epoch != transaction.planned_runtime_epoch {
            return Err(self.resume_transaction_failure(&mut manifest, RESUME_EVIDENCE_MISMATCH));
        }
        self.ensure_restoring_manifest(&mut manifest).await?;
        self.continue_planned_runtime(manifest, transaction, runtime)
            .await
    }

    pub(crate) async fn ensure_restoring_manifest(
        &self,
        manifest: &mut TerminalManifest,
    ) -> std::result::Result<(), SessionResumeFailure> {
        let command = shell_command(Path::new(&manifest.cwd));
        let changed = manifest.recovery_state != RecoveryState::Restoring
            || manifest.command != command.executable.to_string_lossy()
            || manifest.args != command.args;
        if changed {
            manifest.command = command.executable.to_string_lossy().into_owned();
            manifest.args = command.args;
            manifest.recovery_state = RecoveryState::Restoring;
            manifest.updated_at_ms = now_ms();
            self.persist_resume_manifest(manifest).await?;
        }
        Ok(())
    }

    pub(crate) async fn replay_resume_launch(
        &self,
        mut manifest: TerminalManifest,
        transaction: ResumeTransaction,
        launch: WorkerLaunchSpec,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        match self
            .spawn_published_worker(&manifest, &launch, UnstartedLaunchPolicy::Preserve)
            .await
        {
            Ok(WorkerStart::Ready) => {
                self.finish_or_retain_planned_resume(manifest, &transaction)
                    .await
            }
            Ok(WorkerStart::Uncertain(error)) => {
                self.mark_worker_start_uncertain(&mut manifest, &error)
                    .await;
                self.park_resume_retry(
                    manifest.terminal_id,
                    ResumeRetryDisposition::EvidenceRequired,
                )
                .map_err(internal)?;
                Ok(self.view_for_manifest(manifest).await)
            }
            Err(error) => {
                self.persist_recovery_state(&mut manifest, RecoveryState::Uncertain)
                    .await;
                self.park_resume_retry(
                    manifest.terminal_id,
                    ResumeRetryDisposition::ExplicitAllowed,
                )
                .map_err(internal)?;
                Err(worker_start_failure(manifest.terminal_id, &error).into())
            }
        }
    }

    pub(crate) async fn continue_planned_runtime(
        &self,
        mut manifest: TerminalManifest,
        transaction: ResumeTransaction,
        runtime: RuntimeRecord,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        self.validate_planned_runtime(&manifest, &transaction, &runtime)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        match runtime.bootstrap_delivery {
            BootstrapDelivery::Pending if pending_bootstrap_identities_live(&runtime) => match self
                .await_worker_start(
                    &manifest,
                    transaction.planned_runtime_epoch,
                    transaction.worker_id,
                )
                .await
            {
                Ok(WorkerStart::Ready) => {
                    self.finish_or_retain_planned_resume(manifest, &transaction)
                        .await
                }
                Ok(WorkerStart::Uncertain(error)) => {
                    self.mark_worker_start_uncertain(&mut manifest, &error)
                        .await;
                    Ok(self.view_for_manifest(manifest).await)
                }
                Err(error) => {
                    self.persist_recovery_state_durable(&mut manifest, RecoveryState::Uncertain);
                    Err(worker_start_failure(manifest.terminal_id, &error).into())
                }
            },
            BootstrapDelivery::Pending => {
                Err(self.resume_transaction_failure(&mut manifest, RESUME_EVIDENCE_MISMATCH))
            }
            BootstrapDelivery::NotRequired | BootstrapDelivery::Delivered => {
                self.finish_planned_resume(manifest, &transaction).await
            }
            BootstrapDelivery::Survivor => {
                self.retain_planned_resume_survivor(manifest, &transaction, runtime)
                    .await
            }
            BootstrapDelivery::Failed | BootstrapDelivery::Unknown => {
                Err(self.resume_transaction_failure(&mut manifest, RESUME_EVIDENCE_MISMATCH))
            }
        }
    }
}
