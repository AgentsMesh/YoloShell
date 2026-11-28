const RECREATE_SELECTION_REQUIRED: &str = "explicit recreation requires an exact catalog selection";

impl Host {
    pub(super) async fn resume_session(
        &self,
        mut payload: ResumeRequest,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        let terminal_id = payload.selection.terminal_id;
        let _session_guard = self.session_locks.lock(terminal_id).await;
        let ledger_path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        self.recover_session_creation(&ledger_path, terminal_id)?;
        if lock_recover(&self.creation_capabilities)
            .active_creation(terminal_id)?
            .is_some()
        {
            return Err(ErrorBody::conflict(RESUME_CREATION_ACTIVE));
        }
        if payload.recreate && !payload.selection.is_exact_catalog_selection() {
            return Err(ErrorBody::conflict(RECREATE_SELECTION_REQUIRED));
        }
        payload.selection = self.durable_product_selection(payload.selection).await?;
        if payload.selection.is_conditional() || payload.selection.expected_runtime_epoch.is_some()
        {
            self.validate_runtime_epoch_fence(&payload.selection)?;
            self.validate_product_selection(&payload.selection).await?;
        }
        self.resume_session_locked_with_product_selection(
            terminal_id,
            ResumeSelection::explicit(payload.recreate),
            Some(payload.selection),
        )
        .await
        .map_err(SessionResumeFailure::into_explicit_error)
    }
}
