impl Host {
    pub(crate) async fn proxy_worker_request_to(
        &self,
        record: &RuntimeRecord,
        request: &RequestEnvelope,
    ) -> std::result::Result<ResponseEnvelope, ErrorBody> {
        let mut response = self
            .custodied_worker_rpc(record, &request.method, &request.payload)
            .await
            .map_err(WorkerRpcError::into_error_body)?;
        response.request_id = request.request_id.clone();
        response.method = request.method.clone();
        Ok(response)
    }

    pub(crate) async fn proxy_runtime_bound_worker_request(
        &self,
        request: &RequestEnvelope,
    ) -> std::result::Result<ResponseEnvelope, ErrorBody> {
        let (terminal_id, expected_epoch) = extract_runtime_target(&request.payload)?;
        let _session_guard = self.session_locks.lock(terminal_id).await;
        let record = if request.method == method::TERMINAL_INPUT {
            self.runtime_record_for_input(terminal_id).await?
        } else {
            self.live_runtime_record(terminal_id).await?
        };
        if record.runtime_epoch != expected_epoch {
            return Err(ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                REQUEST_RUNTIME_EPOCH_MISMATCH,
                false,
            ));
        }
        if let Some(expected_exit) = self.released_natural_attachment_exit(&record) {
            return self
                .proxy_completed_runtime_request_to(&record, request, &expected_exit)
                .await;
        }
        self.proxy_worker_request_to(&record, request).await
    }

    pub(crate) async fn live_runtime_record(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<RuntimeRecord, ErrorBody> {
        let record: RuntimeRecord = self
            .read_namespace_json(&self.paths.runtime_record(terminal_id))
            .map_err(|_| {
                ErrorBody::new(
                    crate::protocol::error_code::WORKER_UNREACHABLE,
                    "session has no live runtime record",
                    true,
                )
            })?;
        validate_live_worker(&record, terminal_id, &self.boot_id)?;
        Ok(record)
    }
}

include!("worker_rpc_views.rs");


pub(crate) fn runtime_epochs_pair(observed: Option<Uuid>, authority: Option<Uuid>) -> bool {
    match (observed, authority) {
        (Some(observed), Some(authority)) => observed == authority,
        (Some(_), None) => false,
        (None, _) => true,
    }
}
include!("worker_control.rs");
include!("worker_control_released.rs");
include!("worker_rpc_completed.rs");
