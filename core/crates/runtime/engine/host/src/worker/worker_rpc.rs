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
        if record.schema_version != RUNTIME_RECORD_SCHEMA_VERSION {
            return Err(ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                "runtime record schema cannot establish process identity",
                false,
            ));
        }
        if record.terminal_id != terminal_id {
            return Err(ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                "runtime session identity mismatch",
                false,
            ));
        }
        validate_runtime_bootstrap(record.bootstrap_delivery)?;
        if process::boot_identity_relation(&record.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different
        {
            return Err(ErrorBody::new(
                crate::protocol::error_code::HOST_REBOOTED,
                "runtime belongs to a previous host boot",
                false,
            ));
        }
        if record.worker_start_time == 0 || record.child_start_time == 0 {
            return Err(ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                "runtime record contains an unverifiable process identity",
                false,
            ));
        }
        if !process::is_process_alive(record.worker_pid) {
            return Err(ErrorBody::new(
                crate::protocol::error_code::WORKER_UNREACHABLE,
                "session worker is not running",
                true,
            ));
        }
        if record.worker_start_time != 0 {
            let actual = process::process_start_time(record.worker_pid).map_err(|_| {
                ErrorBody::new(
                    crate::protocol::error_code::WORKER_UNREACHABLE,
                    "cannot validate worker process identity",
                    true,
                )
            })?;
            if actual != record.worker_start_time {
                return Err(ErrorBody::new(
                    crate::protocol::error_code::IDENTITY_MISMATCH,
                    "worker PID was reused by another process",
                    false,
                ));
            }
        }
        Ok(record)
    }
}

include!("worker_rpc_views.rs");


/// A binding may be used in a view only when it was validated against the
/// same runtime incarnation the view observed. A view with no runtime
/// observation has its own completed-session presentation; a runtime whose
/// authority vanished is rejected.
pub(crate) fn runtime_epochs_pair(observed: Option<Uuid>, authority: Option<Uuid>) -> bool {
    match (observed, authority) {
        (Some(observed), Some(authority)) => observed == authority,
        // The runtime record disappeared while the binding was being loaded;
        // the view's runtime no longer has validated agent authority.
        (Some(_), None) => false,
        (None, _) => true,
    }
}
include!("worker_control.rs");
include!("worker_control_released.rs");
include!("worker_rpc_completed.rs");
