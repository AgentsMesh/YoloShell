impl Host {
    pub(crate) async fn cleanup_archive_session(
        &self,
        request: CreationCleanupArchiveRequest,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        let _session_guard = self.session_locks.lock(request.terminal_id).await;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        let state = lock_recover(&self.creation_capabilities).cleanup_state(
            &path,
            request.terminal_id,
            request.creation_capability,
            &self.creation_receipts,
        )?;
        match state {
            CleanupCreationState::Terminated => {
                self.wait_for_session_exit(request.terminal_id).await?;
                let view = self.archive_bound_session(request.terminal_id).await?;
                lock_recover(&self.creation_capabilities).complete_archive(
                    &path,
                    lifecycle_result(&request),
                    &self.creation_receipts,
                )?;
                Ok(view)
            }
            CleanupCreationState::Archived => self.view_for_session(request.terminal_id).await,
            CleanupCreationState::Ready => Err(ErrorBody::conflict(
                "creation must be cleanup-terminated before cleanup-archive",
            )),
            CleanupCreationState::Finalized => {
                Err(ErrorBody::conflict(CLEANUP_FINALIZED_UNAUTHORIZED))
            }
            CleanupCreationState::Survivor => {
                Err(ErrorBody::conflict(CLEANUP_SURVIVOR_ARCHIVE_UNAUTHORIZED))
            }
        }
    }

    pub(crate) async fn cleanup_discard_session(
        &self,
        request: CreationCleanupDiscardRequest,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        let _session_guard = self.session_locks.lock(request.terminal_id).await;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        let mut state = lock_recover(&self.creation_capabilities).cleanup_state(
            &path,
            request.terminal_id,
            request.creation_capability,
            &self.creation_receipts,
        )?;
        if state == CleanupCreationState::Ready {
            state = match self
                .settle_cleanup_termination(
                    &path,
                    discard_result(&request),
                    request.terminal_id,
                    request.force,
                )
                .await?
            {
                CleanupTerminationOutcome::Terminated => CleanupCreationState::Terminated,
                CleanupTerminationOutcome::Survivor => {
                    return self.view_for_session(request.terminal_id).await;
                }
            };
        }
        match state {
            CleanupCreationState::Terminated => {
                self.wait_for_session_exit(request.terminal_id).await?;
                let view = self.archive_bound_session(request.terminal_id).await?;
                lock_recover(&self.creation_capabilities).complete_archive(
                    &path,
                    discard_result(&request),
                    &self.creation_receipts,
                )?;
                Ok(view)
            }
            CleanupCreationState::Archived => self.view_for_session(request.terminal_id).await,
            CleanupCreationState::Finalized => {
                Err(ErrorBody::conflict(CLEANUP_FINALIZED_UNAUTHORIZED))
            }
            CleanupCreationState::Survivor => self.view_for_session(request.terminal_id).await,
            CleanupCreationState::Ready => unreachable!("{CLEANUP_READY_TRANSITION_INVARIANT}"),
        }
    }

    pub(crate) async fn finalize_creation(
        &self,
        request: FinalizeCreationRequest,
    ) -> std::result::Result<(), ErrorBody> {
        let _session_guard = self.session_locks.lock(request.terminal_id).await;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        let pending = lock_recover(&self.creation_capabilities)
            .pending_finalize(&request, &self.creation_receipts)?;
        if let Some(binding) = &pending {
            self.ensure_created_session_exists(binding).await?;
        }
        lock_recover(&self.creation_capabilities).finalize(&path, request, &self.creation_receipts)
    }

    pub(crate) async fn view_for_session(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        let manifest = self
            .catalog_get(terminal_id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        Ok(self.view_for_manifest(manifest).await)
    }
}

pub(crate) fn cleanup_result(request: &CreationCleanupTerminateRequest) -> CreateTerminalResult {
    CreateTerminalResult {
        terminal_id: request.terminal_id,
        creation_capability: request.creation_capability,
    }
}

pub(crate) fn lifecycle_result(request: &CreationCleanupArchiveRequest) -> CreateTerminalResult {
    CreateTerminalResult {
        terminal_id: request.terminal_id,
        creation_capability: request.creation_capability,
    }
}

pub(crate) fn discard_result(request: &CreationCleanupDiscardRequest) -> CreateTerminalResult {
    CreateTerminalResult {
        terminal_id: request.terminal_id,
        creation_capability: request.creation_capability,
    }
}
