use super::*;

pub(super) async fn dispatch_request(
    request: &RequestEnvelope,
    state: &Arc<WorkerState>,
    controller: AuthenticatedController,
) -> ResponseEnvelope {
    if let Err(error) = state.namespace.verify() {
        return ResponseEnvelope::failure(request, internal_error(error));
    }
    let response = match dispatch_request_inner(request, state, controller).await {
        Ok(response) => response,
        Err(error) => ResponseEnvelope::failure(request, error),
    };
    match state.namespace.verify() {
        Ok(()) => response,
        Err(error) => ResponseEnvelope::failure(request, internal_error(error)),
    }
}

pub(super) async fn dispatch_request_inner(
    request: &RequestEnvelope,
    state: &Arc<WorkerState>,
    controller: AuthenticatedController,
) -> std::result::Result<ResponseEnvelope, ErrorBody> {
    state.validate_controller(controller)?;
    match request.method.as_str() {
        crate::protocol::method::OBSERVE_NATIVE_AGENT
        | crate::protocol::method::GET_NATIVE_CONTROL_RECEIPT
        | crate::protocol::method::INTERRUPT_NATIVE_AGENT
        | crate::protocol::method::SUBMIT_NATIVE_AGENT
        | crate::protocol::method::RESPOND_NATIVE_AGENT => {
            native_control::dispatch(request, state, controller).await
        }
        WORKER_STATUS_METHOD => {
            ResponseEnvelope::success(request, &state.status()).map_err(protocol_error)
        }
        WORKER_SHUTDOWN_METHOD => {
            let payload: TerminalRef = decode_worker_payload(request)?;
            state.validate_session(payload.terminal_id)?;
            let _permit = state.admit_controller_mutation(controller).await?;
            if lock_recover(&state.exit).is_none() {
                return Err(ErrorBody::conflict(
                    "cannot retire a worker while its session process is running",
                ));
            }
            state.schedule_controller_shutdown(controller);
            Ok(ResponseEnvelope::empty(request))
        }
        crate::protocol::method::GET_SNAPSHOT => {
            let payload: SnapshotRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            let permit = state.admit_controller_mutation(controller).await?;
            state.ensure_snapshot_or_attachment_allowed()?;
            let snapshot = state
                .current_snapshot_admitted(&permit)
                .map_err(internal_error)?;
            ResponseEnvelope::success(request, &snapshot).map_err(protocol_error)
        }
        crate::protocol::method::GET_TEXT => {
            let payload: TerminalTextRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            let permit = state.admit_controller_mutation(controller).await?;
            state.ensure_snapshot_or_attachment_allowed()?;
            let text = state
                .current_text_admitted(&permit)
                .map_err(internal_error)?;
            ResponseEnvelope::success(request, &text).map_err(protocol_error)
        }
        crate::protocol::method::ACQUIRE_WRITER | crate::protocol::method::TAKEOVER_WRITER => {
            let payload: LeaseRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            let takeover = request.method == crate::protocol::method::TAKEOVER_WRITER;
            let lease = state.acquire_lease(controller, &payload, takeover).await?;
            ResponseEnvelope::success(request, &lease).map_err(protocol_error)
        }
        crate::protocol::method::GET_WRITER_LEASE => {
            let payload: RuntimeRef = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            let lease = state.get_writer_lease(controller).await?;
            ResponseEnvelope::success(request, &lease).map_err(protocol_error)
        }
        crate::protocol::method::RENEW_WRITER => {
            let payload: LeaseRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            let lease = state.renew_lease(controller, &payload).await?;
            ResponseEnvelope::success(request, &lease).map_err(protocol_error)
        }
        crate::protocol::method::RELEASE_WRITER => {
            let payload: LeaseRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            state.release_lease(controller, &payload).await?;
            Ok(ResponseEnvelope::empty(request))
        }
        crate::protocol::method::TERMINAL_INPUT => {
            let payload: InputRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            let result = state.write_input(controller, &payload).await?;
            ResponseEnvelope::success(request, &result).map_err(protocol_error)
        }
        crate::protocol::method::RESIZE_TERMINAL => {
            let payload: ResizeRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            state.resize_terminal(controller, &payload).await?;
            Ok(ResponseEnvelope::empty(request))
        }
        crate::protocol::method::SIGNAL_SESSION => {
            let payload: SignalRequest = decode_worker_payload(request)?;
            state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
            let permit = state
                .admit_writer_mutation(controller, payload.generation, &payload.writer_capability)
                .await?;
            let signal = PtyForegroundSignal::parse(&payload.signal)
                .map_err(|error| ErrorBody::invalid(error.to_string()))?;
            signal_pty_foreground(state, &permit, signal)
                .await
                .map_err(internal_error)?;
            Ok(ResponseEnvelope::empty(request))
        }
        crate::protocol::method::TERMINATE_RUNTIME_EXACT => {
            let payload: FencedTerminateRequest = decode_worker_payload(request)?;
            let permit = state.admit_controller_mutation(controller).await?;
            state.ensure_termination_allowed()?;
            validate_exact_termination_admitted(state, &permit, &payload)?;
            terminate_runtime_admitted(
                state,
                controller,
                permit,
                payload.terminal_id,
                payload.force,
            )
            .await?;
            Ok(ResponseEnvelope::empty(request))
        }
        _ => Err(ErrorBody::new(
            crate::protocol::error_code::UNSUPPORTED_METHOD,
            format!("worker does not support {}", request.method),
            false,
        )),
    }
}
