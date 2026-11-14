include!("worker_retirement.rs");

pub(crate) const WORKER_STATUS_IDENTITY_MISMATCH: &str =
    "worker status identity does not match its authenticated runtime";
pub(crate) const WORKER_STATUS_CAPABILITY_MISMATCH: &str =
    "worker status capabilities do not match its authenticated hello";
pub(crate) const WORKER_STATUS_CUSTODY_UNAVAILABLE: &str =
    "worker status requires established exact PTY custody";

impl Host {
    pub(crate) async fn worker_status(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<RuntimeView, WorkerRpcError> {
        // Custody can cross a fail-stop boundary, so only the status RPC is cancellable here.
        self.require_rpc_custody(record).await?;
        self.worker_status_rpc(record).await
    }

    pub(crate) async fn worker_status_if_custodied(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<RuntimeView, WorkerRpcError> {
        if !self.rpc_custody_ready(record) {
            return Err(WorkerRpcError::transport(WORKER_STATUS_CUSTODY_UNAVAILABLE));
        }
        self.worker_status_rpc(record).await
    }

    pub(crate) async fn worker_status_rpc(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<RuntimeView, WorkerRpcError> {
        let (response, authenticated) = tokio::time::timeout(
            self.worker_status_timeout,
            self.authenticated_worker_rpc(
                record,
                WORKER_STATUS_METHOD,
                &TerminalRef {
                    terminal_id: record.terminal_id,
                },
            ),
        )
        .await
        .map_err(|_| WorkerRpcError::transport(WORKER_STATUS_TIMEOUT_DETAIL))??;
        if !response.ok {
            return Err(response
                .error
                .map(WorkerRpcError::remote)
                .unwrap_or_else(|| WorkerRpcError::protocol(WORKER_STATUS_FAILED)));
        }
        let status: RuntimeView = response
            .decode_payload()
            .map_err(WorkerRpcError::from_protocol)?;
        if !runtime_view_matches(&status, &authenticated.runtime) {
            return Err(WorkerRpcError::identity_mismatch(
                WORKER_STATUS_IDENTITY_MISMATCH,
            ));
        }
        if status.capabilities.as_ref() != Some(&authenticated.capabilities) {
            return Err(WorkerRpcError::identity_mismatch(
                WORKER_STATUS_CAPABILITY_MISMATCH,
            ));
        }
        Ok(status)
    }

    pub(crate) async fn retire_worker_classified(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<(), WorkerRetirementError> {
        let response = self
            .custodied_worker_rpc(
                record,
                WORKER_SHUTDOWN_METHOD,
                &TerminalRef {
                    terminal_id: record.terminal_id,
                },
            )
            .await
            .map_err(|error| {
                WorkerRetirementError::control(WorkerRetirementControlPhase::CustodiedRpc, error)
            })?;
        self.finish_worker_retirement(record, response).await
    }

    pub(crate) async fn finish_worker_retirement(
        &self,
        record: &RuntimeRecord,
        response: ResponseEnvelope,
    ) -> std::result::Result<(), WorkerRetirementError> {
        self.worker_connections.clear(record.terminal_id).await;
        if !response.ok {
            let error = response
                .error
                .map(WorkerRpcError::remote)
                .unwrap_or_else(|| WorkerRpcError::protocol(WORKER_RETIREMENT_FAILED));
            return Err(WorkerRetirementError::control(
                WorkerRetirementControlPhase::ShutdownResponse,
                error,
            ));
        }
        if !wait_for_process_identity_exit(
            record.worker_pid,
            record.worker_start_time,
            PROCESS_EXIT_TIMEOUT,
        )
        .await
        {
            return Err(WorkerRetirementError::ExitTimeout);
        }
        if socket_may_be_serving(&record.socket_path).await {
            return Err(WorkerRetirementError::SocketServing);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) async fn retire_worker(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<(), ErrorBody> {
        self.retire_worker_classified(record)
            .await
            .map_err(WorkerRetirementError::into_error_body)
    }
}

pub(crate) fn runtime_view_matches(status: &RuntimeView, runtime: &RuntimeRecord) -> bool {
    status.runtime_epoch == runtime.runtime_epoch
        && status.worker_id == runtime.worker_id
        && status.worker_pid == runtime.worker_pid
        && status.worker_start_time == runtime.worker_start_time
        && status.child_pid == runtime.child_pid
        && status.child_start_time == runtime.child_start_time
}

pub(crate) fn worker_unreachable(detail: impl Into<String>) -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::WORKER_UNREACHABLE,
        detail,
        true,
    )
}
