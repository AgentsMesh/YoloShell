pub(crate) const RECONCILE_RESUME_MANIFEST_MISMATCH: &str =
    "automatic resume failure manifest does not match the session";
pub(crate) const RECONCILE_RESUME_MANIFEST_ARCHIVED: &str =
    "automatic resume failure manifest was archived concurrently";
pub(crate) const RECONCILE_TERMINAL_STATE_MISSING: &str =
    "terminal automatic resume failure has no durable recreation state";

impl Host {
    pub(crate) async fn reconcile_resume_failure(
        &self,
        terminal_id: Uuid,
        failure: SessionResumeFailure,
    ) -> bool {
        match failure {
            SessionResumeFailure::TerminalRecreate(error) => {
                tracing::warn!(
                    %terminal_id,
                    code = %error.code,
                    detail = %error.message,
                    "automatic session recovery requires explicit recreation"
                );
                match self.confirm_terminal_resume_failure(terminal_id).await {
                    Ok(()) => true,
                    Err(error) => {
                        tracing::error!(
                            %terminal_id,
                            %error,
                            "terminal automatic resume failure was not durably confirmed"
                        );
                        false
                    }
                }
            }
            SessionResumeFailure::Protocol(error) => {
                tracing::warn!(
                    %terminal_id,
                    code = %error.code,
                    detail = %error.message,
                    "automatic session recovery failed"
                );
                false
            }
            SessionResumeFailure::AutomaticBlocked(block) => {
                tracing::warn!(
                    %terminal_id,
                    ?block,
                    "automatic agent recovery failed closed"
                );
                if block == AutomaticResumeBlock::RecoveryEvidenceChanged {
                    if let Err(error) = self.converge_automatic_resume_block(terminal_id).await {
                        tracing::error!(
                            %terminal_id,
                            %error,
                            "automatic recovery block did not converge"
                        );
                    }
                }
                false
            }
        }
    }

    pub(crate) async fn confirm_terminal_resume_failure(&self, terminal_id: Uuid) -> Result<()> {
        let path = self.paths.manifest(terminal_id);
        let (manifest, identity) = self.read_reconcile_resume_manifest(terminal_id)?;
        anyhow::ensure!(
            manifest.recovery_state == RecoveryState::NeedsRecreate,
            RECONCILE_TERMINAL_STATE_MISSING
        );
        self.catalog_upsert_expected_manifest(&manifest, path, identity)
            .await
    }

    pub(crate) async fn converge_automatic_resume_block(&self, terminal_id: Uuid) -> Result<()> {
        let path = self.paths.manifest(terminal_id);
        let (manifest, identity) = self.read_reconcile_resume_manifest(terminal_id)?;
        if !matches!(
            manifest.recovery_state,
            RecoveryState::Created
                | RecoveryState::Reattached
                | RecoveryState::Restoring
                | RecoveryState::Uncertain
        ) {
            return Ok(());
        }
        let changed = manifest.recovery_state != RecoveryState::Uncertain;
        let mut candidate = manifest.clone();
        let installed_identity = if changed {
            candidate.recovery_state = RecoveryState::Uncertain;
            candidate.updated_at_ms = now_ms();
            recovery_manifest_publication::PreparedRecoveryManifest::capture_observed(
                self, &manifest, identity, &candidate,
            )?
            .commit(self)?
        } else {
            identity
        };
        self.catalog_upsert_expected_manifest(&candidate, path, installed_identity)
            .await?;
        #[cfg(test)]
        self.notify_recovery_publication(&candidate);
        Ok(())
    }

    pub(crate) fn read_reconcile_resume_manifest(
        &self,
        terminal_id: Uuid,
    ) -> Result<(TerminalManifest, crate::paths::identity::FileIdentity)> {
        let path = self.paths.manifest(terminal_id);
        let (manifest, identity): (TerminalManifest, crate::paths::identity::FileIdentity) =
            self.namespace.read_json_with_identity(&path)?;
        anyhow::ensure!(
            manifest.schema_version == SESSION_MANIFEST_SCHEMA_VERSION
                && manifest.terminal_id == terminal_id,
            RECONCILE_RESUME_MANIFEST_MISMATCH
        );
        anyhow::ensure!(!manifest.archived, RECONCILE_RESUME_MANIFEST_ARCHIVED);
        Ok((manifest, identity))
    }
}
