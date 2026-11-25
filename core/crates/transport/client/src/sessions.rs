use super::deadline::ClientDeadline;
use super::*;

#[path = "sessions/text.rs"]
mod text;
#[path = "sessions/history.rs"]
mod history;

impl Client {
    pub async fn status(&self) -> Result<HostStatus, ClientError> {
        self.status_until(self.operation_deadline()).await
    }

    pub(super) async fn status_until(
        &self,
        deadline: ClientDeadline,
    ) -> Result<HostStatus, ClientError> {
        let mut connection = self.connect_until(deadline).await?;
        match connection.take_authenticated_status() {
            Some(status) => Ok(status),
            None => {
                connection
                    .request_until(method::HOST_STATUS, &(), deadline)
                    .await
            }
        }
    }

    pub async fn get_session(&self, terminal_id: Uuid) -> Result<TerminalView, ClientError> {
        self.request(method::GET_SESSION, &TerminalRef { terminal_id })
            .await
    }

    pub async fn create_session(
        &self,
        request: &CreateTerminal,
    ) -> Result<CreateTerminalResult, ClientError> {
        self.request(method::CREATE_SESSION, request).await
    }

    pub async fn terminate(&self, terminal_id: Uuid, force: bool) -> Result<(), ClientError> {
        self.terminate_with_preconditions(TerminalLifecycleSelection::unconditional(terminal_id), force)
            .await
    }

    pub async fn snapshot(
        &self,
        terminal_id: Uuid,
    ) -> Result<crate::model::TerminalSnapshot, ClientError> {
        self.snapshot_until(terminal_id, self.operation_deadline())
            .await
    }

    pub(super) async fn snapshot_until(
        &self,
        terminal_id: Uuid,
        deadline: ClientDeadline,
    ) -> Result<crate::model::TerminalSnapshot, ClientError> {
        let session: TerminalView = self
            .request_until(method::GET_SESSION, &TerminalRef { terminal_id }, deadline)
            .await?;
        let completed_barrier = completed_snapshot::expected_barrier(&session)?;
        let runtime = session
            .runtime
            .ok_or(ClientError::RuntimeUnavailable { terminal_id })?;
        let snapshot: crate::model::TerminalSnapshot = self
            .request_until(
                method::GET_SNAPSHOT,
                &SnapshotRequest {
                    terminal_id,
                    runtime_epoch: runtime.runtime_epoch,
                },
                deadline,
            )
            .await?;
        if snapshot.runtime_epoch != runtime.runtime_epoch
            || completed_barrier.is_some_and(|barrier| snapshot.barrier_sequence != barrier)
        {
            return Err(ProtocolError::AttachmentMismatch.into());
        }
        Ok(snapshot)
    }

    pub async fn archive(&self, terminal_id: Uuid) -> Result<TerminalView, ClientError> {
        self.archive_with_preconditions(TerminalLifecycleSelection::unconditional(terminal_id))
            .await
    }

    pub async fn discard(&self, terminal_id: Uuid, force: bool) -> Result<TerminalView, ClientError> {
        self.discard_with_preconditions(TerminalLifecycleSelection::unconditional(terminal_id), force)
            .await
    }

    pub async fn cleanup_terminate(
        &self,
        terminal_id: Uuid,
        creation_capability: Uuid,
        force: bool,
    ) -> Result<CleanupTerminationOutcome, ClientError> {
        self.request(
            method::CLEANUP_TERMINATE_SESSION,
            &CreationCleanupTerminateRequest {
                terminal_id: terminal_id,
                creation_capability,
                force,
            },
        )
        .await
    }

    pub async fn cleanup_archive(
        &self,
        terminal_id: Uuid,
        creation_capability: Uuid,
    ) -> Result<TerminalView, ClientError> {
        self.request(
            method::CLEANUP_ARCHIVE_SESSION,
            &CreationCleanupArchiveRequest {
                terminal_id: terminal_id,
                creation_capability,
            },
        )
        .await
    }

    pub async fn cleanup_discard(
        &self,
        terminal_id: Uuid,
        creation_capability: Uuid,
        force: bool,
    ) -> Result<TerminalView, ClientError> {
        self.request(
            method::CLEANUP_DISCARD_SESSION,
            &CreationCleanupDiscardRequest {
                terminal_id: terminal_id,
                creation_capability,
                force,
            },
        )
        .await
    }

    pub async fn finalize_creation(
        &self,
        creation_token: Uuid,
        terminal_id: Uuid,
        creation_capability: Uuid,
    ) -> Result<(), ClientError> {
        self.request(
            method::FINALIZE_CREATION,
            &FinalizeCreationRequest {
                creation_token,
                terminal_id: terminal_id,
                creation_capability,
            },
        )
        .await
    }

    pub async fn resume(
        &self,
        terminal_id: Uuid,
        recreate: bool,
    ) -> Result<TerminalView, ClientError> {
        let request = ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(terminal_id),
            recreate,
        };
        self.request(method::RESUME_SESSION_EXACT, &request).await
    }

    pub async fn resume_with_preconditions(
        &self,
        selection: TerminalLifecycleSelection,
        recreate: bool,
    ) -> Result<TerminalView, ClientError> {
        self.request(
            method::RESUME_SESSION_EXACT,
            &ResumeRequest {
                selection,
                recreate,
            },
        )
        .await
    }

    pub async fn reconcile(&self) -> Result<ReconcileResult, ClientError> {
        self.request(method::RECONCILE, &()).await
    }
}
