impl Host {
    pub(crate) async fn retire_released_worker_classified(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<(), WorkerRetirementError> {
        self.validate_released_shutdown_for_retirement(
            record,
            WorkerRetirementControlPhase::ReleasedEvidenceBeforeAuth,
        )?;
        let mut stream = authenticated_stream(record, self.controller_generation)
            .await
            .map_err(|error| {
                WorkerRetirementError::control(WorkerRetirementControlPhase::Authentication, error)
            })?;
        self.validate_released_shutdown_for_retirement(
            record,
            WorkerRetirementControlPhase::ReleasedEvidenceAfterAuth,
        )?;
        let payload = crate::protocol::to_value(&TerminalRef {
            terminal_id: record.terminal_id,
        })
        .map_err(|error| {
            WorkerRetirementError::control(
                WorkerRetirementControlPhase::ShutdownEncoding,
                WorkerRpcError::protocol(error),
            )
        })?;
        let request =
            RequestEnvelope::new(Uuid::new_v4().to_string(), WORKER_SHUTDOWN_METHOD, payload);
        let response = exchange_worker_request(&mut stream, &request)
            .await
            .map_err(|error| {
                WorkerRetirementError::control(
                    WorkerRetirementControlPhase::ShutdownExchange,
                    error,
                )
            })?;
        drop(stream);
        self.finish_worker_retirement(record, response).await
    }

    pub(crate) fn validate_released_shutdown_for_retirement(
        &self,
        record: &RuntimeRecord,
        phase: WorkerRetirementControlPhase,
    ) -> std::result::Result<(), WorkerRetirementError> {
        self.validate_released_terminal_worker_shutdown(record)
            .map_err(|error| WorkerRetirementError::control(phase, WorkerRpcError::remote(error)))
    }
}
