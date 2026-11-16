impl Host {
    pub(crate) async fn proxy_completed_runtime_request_to(
        &self,
        record: &RuntimeRecord,
        request: &RequestEnvelope,
        expected_exit: &ExitRecord,
    ) -> std::result::Result<ResponseEnvelope, ErrorBody> {
        if request.method != method::GET_SNAPSHOT {
            return Err(ErrorBody::new(
                crate::lease::constants::code::SESSION_NOT_RUNNING,
                COMPLETED_RUNTIME_NOT_RUNNING,
                false,
            ));
        }
        self.require_completed_runtime_exit(record, expected_exit)
            .await?;
        let mut stream = authenticated_stream(record, self.controller_generation)
            .await
            .map_err(WorkerRpcError::into_error_body)?;
        let mut response = exchange_worker_request(&mut stream, request)
            .await
            .map_err(WorkerRpcError::into_error_body)?;
        validate_completed_snapshot(&response, expected_exit)?;
        self.require_completed_runtime_exit(record, expected_exit)
            .await?;
        response.request_id = request.request_id.clone();
        response.method = request.method.clone();
        Ok(response)
    }
}

pub(crate) fn validate_completed_snapshot(
    response: &ResponseEnvelope,
    expected_exit: &ExitRecord,
) -> std::result::Result<(), ErrorBody> {
    if !response.ok {
        return Ok(());
    }
    let snapshot: crate::model::TerminalSnapshot = response
        .decode_payload()
        .map_err(|_| ErrorBody::protocol_incompatible(COMPLETED_SNAPSHOT_ATTESTATION_MISMATCH))?;
    if snapshot.runtime_epoch == expected_exit.runtime_epoch
        && snapshot.barrier_sequence == expected_exit.last_output_sequence
    {
        return Ok(());
    }
    Err(ErrorBody::new(
        crate::protocol::error_code::IDENTITY_MISMATCH,
        COMPLETED_SNAPSHOT_ATTESTATION_MISMATCH,
        false,
    ))
}
