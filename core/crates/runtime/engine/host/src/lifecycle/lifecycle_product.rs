impl Host {
    #[cfg(test)]
    pub(crate) async fn terminate_session(
        &self,
        request: TerminationIntent,
    ) -> std::result::Result<(), ErrorBody> {
        let _session_guard = self.session_locks.lock(request.terminal_id).await;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        self.recover_session_creation(&path, request.terminal_id)?;
        let active =
            lock_recover(&self.creation_capabilities).active_creation(request.terminal_id)?;
        self.terminate_product_session(&path, request, active, false, None)
            .await
    }

    pub(crate) async fn terminate_product_request(
        &self,
        mut request: TerminalTerminateRequest,
    ) -> std::result::Result<(), ErrorBody> {
        let terminal_id = request.selection.terminal_id;
        let _session_guard = self.session_locks.lock(terminal_id).await;
        request.selection = self.durable_product_selection(request.selection).await?;
        self.validate_runtime_epoch_fence(&request.selection)?;
        self.validate_stop_selection(&request.selection).await?;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        self.recover_session_creation(&path, terminal_id)?;
        let active = lock_recover(&self.creation_capabilities).active_creation(terminal_id)?;
        let (lifecycle_lock, fence) = self
            .acquire_lifecycle_mutation_fence(&request.selection)
            .await?;
        drop(lifecycle_lock);
        self.terminate_product_session(
            &path,
            TerminationIntent {
                terminal_id,
                force: request.force,
            },
            active,
            false,
            Some(&fence),
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn archive_session(
        &self,
        request: TerminalRef,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        self.archive_product_session(TerminalLifecycleSelection::unconditional(request.terminal_id))
            .await
    }

    pub(crate) async fn archive_product_session(
        &self,
        request: TerminalLifecycleSelection,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        let _session_guard = self.session_locks.lock(request.terminal_id).await;
        let request = self.durable_product_selection(request).await?;
        self.validate_runtime_epoch_fence(&request)?;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        eligible_product_archive(
            lock_recover(&self.creation_capabilities).active_creation(request.terminal_id)?,
        )
        .map(|_| ())?;
        self.validate_product_selection(&request).await?;
        self.recover_session_creation(&path, request.terminal_id)?;
        let active = eligible_product_archive(
            lock_recover(&self.creation_capabilities).active_creation(request.terminal_id)?,
        )?;
        let view = self.archive_selected_session(&request).await?;
        if let Some(result) = active {
            lock_recover(&self.creation_capabilities).complete_product_archive(
                &path,
                result,
                &self.creation_receipts,
            )?;
        }
        Ok(view)
    }

    #[cfg(test)]
    pub(crate) async fn discard_session(
        &self,
        request: TerminationIntent,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        self.discard_product_session(TerminalDiscardRequest {
            selection: TerminalLifecycleSelection::unconditional(request.terminal_id),
            force: request.force,
        })
        .await
    }

    pub(crate) async fn discard_product_session(
        &self,
        mut request: TerminalDiscardRequest,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        let terminal_id = request.selection.terminal_id;
        let _session_guard = self.session_locks.lock(terminal_id).await;
        request.selection = self.durable_product_selection(request.selection).await?;
        self.validate_runtime_epoch_fence(&request.selection)?;
        eligible_product_archive(
            lock_recover(&self.creation_capabilities).active_creation(terminal_id)?,
        )
        .map(|_| ())?;
        self.validate_product_selection(&request.selection).await?;
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        self.recover_session_creation(&path, terminal_id)?;
        let active = lock_recover(&self.creation_capabilities).active_creation(terminal_id)?;
        let (lifecycle_lock, fence) = self
            .acquire_lifecycle_mutation_fence(&request.selection)
            .await?;
        drop(lifecycle_lock);
        self.terminate_product_session(
            &path,
            TerminationIntent {
                terminal_id,
                force: request.force,
            },
            active,
            lifecycle_discard_inactive_only(&request.selection),
            Some(&fence),
        )
        .await?;
        self.wait_for_session_exit(terminal_id).await?;
        let view = self
            .archive_selected_session_with_retained_fence(&request.selection, &fence)
            .await?;
        if let Some((result, _)) = active {
            lock_recover(&self.creation_capabilities).complete_product_archive(
                &path,
                result,
                &self.creation_receipts,
            )?;
        }
        Ok(view)
    }

    pub(crate) async fn terminate_product_session(
        &self,
        path: &Path,
        request: TerminationIntent,
        active: Option<(CreateTerminalResult, CreationPhase)>,
        inactive_only: bool,
        runtime_fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<(), ErrorBody> {
        match active {
            Some((_, CreationPhase::Terminated)) => Ok(()),
            Some((result, CreationPhase::Ready)) => {
                self.terminate_product_runtime(&request, inactive_only, runtime_fence)
                    .await?;
                lock_recover(&self.creation_capabilities).set_active_phase(
                    path,
                    result,
                    CreationPhase::Terminated,
                )
            }
            Some(_) => Err(ErrorBody::conflict(CREATION_PHASE_TRANSITION_CONFLICT)),
            None => {
                self.terminate_product_runtime(&request, inactive_only, runtime_fence)
                    .await
            }
        }
    }
}

pub(crate) fn eligible_product_archive(
    active: Option<(CreateTerminalResult, CreationPhase)>,
) -> std::result::Result<Option<CreateTerminalResult>, ErrorBody> {
    match active {
        Some((result, phase)) if phase.permits_product_archive() => Ok(Some(result)),
        Some(_) => Err(ErrorBody::conflict(CREATION_PHASE_TRANSITION_CONFLICT)),
        None => Ok(None),
    }
}
pub(crate) fn lifecycle_discard_inactive_only(selection: &TerminalLifecycleSelection) -> bool {
    (selection.expected_runtime_epoch.is_none() && selection.is_conditional())
        || (selection.expected_runtime_epoch.is_some()
            && selection.expected_status == Some(TerminalRuntimeStatus::Exited))
}
