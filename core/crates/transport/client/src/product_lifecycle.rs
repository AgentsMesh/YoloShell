use super::*;

impl Client {
    pub async fn terminate_with_preconditions(
        &self,
        selection: TerminalLifecycleSelection,
        force: bool,
    ) -> Result<(), ClientError> {
        self.request(
            method::TERMINATE_SESSION,
            &TerminalTerminateRequest { selection, force },
        )
        .await
    }

    pub async fn archive_with_preconditions(
        &self,
        selection: TerminalLifecycleSelection,
    ) -> Result<TerminalView, ClientError> {
        self.request(method::ARCHIVE_SESSION, &selection).await
    }

    pub async fn discard_with_preconditions(
        &self,
        selection: TerminalLifecycleSelection,
        force: bool,
    ) -> Result<TerminalView, ClientError> {
        self.request(
            method::DISCARD_SESSION,
            &TerminalDiscardRequest { selection, force },
        )
        .await
    }

    pub async fn discard_if_runtime_epoch(
        &self,
        terminal_id: Uuid,
        force: bool,
        expected_runtime_epoch: Uuid,
    ) -> Result<TerminalView, ClientError> {
        self.discard_with_preconditions(
            TerminalLifecycleSelection::at_runtime_epoch(terminal_id, expected_runtime_epoch),
            force,
        )
        .await
    }

    pub async fn terminate_if_current(
        &self,
        terminal_id: Uuid,
        force: bool,
        updated_at_ms: i64,
        status: TerminalRuntimeStatus,
    ) -> Result<(), ClientError> {
        self.terminate_with_preconditions(
            TerminalLifecycleSelection::selected(terminal_id, updated_at_ms, status),
            force,
        )
        .await
    }

    pub async fn archive_if_current(
        &self,
        terminal_id: Uuid,
        updated_at_ms: i64,
        status: TerminalRuntimeStatus,
    ) -> Result<TerminalView, ClientError> {
        self.archive_with_preconditions(TerminalLifecycleSelection::selected(
            terminal_id,
            updated_at_ms,
            status,
        ))
        .await
    }

    pub async fn discard_if_current(
        &self,
        terminal_id: Uuid,
        force: bool,
        updated_at_ms: i64,
        status: TerminalRuntimeStatus,
    ) -> Result<TerminalView, ClientError> {
        self.discard_with_preconditions(
            TerminalLifecycleSelection::selected(terminal_id, updated_at_ms, status),
            force,
        )
        .await
    }
}
