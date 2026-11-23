impl Host {
    pub(crate) async fn cleanup_terminate_session(
        &self,
        request: CreationCleanupTerminateRequest,
    ) -> std::result::Result<CleanupTerminationOutcome, ErrorBody> {
        let _session_guard = self.session_locks.lock(request.terminal_id).await;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        let state = lock_recover(&self.creation_capabilities).cleanup_state(
            &path,
            request.terminal_id,
            request.creation_capability,
            &self.creation_receipts,
        )?;
        match state {
            CleanupCreationState::Ready => {
                self.settle_cleanup_termination(
                    &path,
                    cleanup_result(&request),
                    request.terminal_id,
                    request.force,
                )
                .await
            }
            CleanupCreationState::Terminated | CleanupCreationState::Archived => {
                Ok(CleanupTerminationOutcome::Terminated)
            }
            CleanupCreationState::Survivor => {
                self.persist_survivor_recovery_state(request.terminal_id)
                    .await?;
                Ok(CleanupTerminationOutcome::Survivor)
            }
            CleanupCreationState::Finalized => {
                Err(ErrorBody::conflict(CLEANUP_FINALIZED_UNAUTHORIZED))
            }
        }
    }

    pub(crate) async fn settle_cleanup_termination(
        &self,
        path: &Path,
        result: CreateTerminalResult,
        terminal_id: Uuid,
        force: bool,
    ) -> std::result::Result<CleanupTerminationOutcome, ErrorBody> {
        match self.terminate_bound_session(terminal_id, force).await {
            Ok(()) => {
                lock_recover(&self.creation_capabilities).set_active_phase(
                    path,
                    result,
                    CreationPhase::Terminated,
                )?;
                Ok(CleanupTerminationOutcome::Terminated)
            }
            Err(error)
                if error.code == crate::protocol::error_code::DESTRUCTIVE_SESSION_UNAVAILABLE =>
            {
                self.persist_survivor_recovery_state(terminal_id).await?;
                lock_recover(&self.creation_capabilities).complete_survivor(
                    path,
                    result,
                    &self.creation_receipts,
                )?;
                Ok(CleanupTerminationOutcome::Survivor)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn persist_survivor_recovery_state(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<(), ErrorBody> {
        let mut manifest = self
            .read_namespace_json::<TerminalManifest>(&self.paths.manifest(terminal_id))
            .map_err(internal)?;
        self.persist_recovery_state_required(&mut manifest, RecoveryState::Uncertain)
            .await
            .map_err(internal)
    }
}
