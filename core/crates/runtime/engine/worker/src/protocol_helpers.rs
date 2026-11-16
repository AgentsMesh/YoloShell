use super::*;

pub(super) const WORKER_AUTH_IDENTITY_MISMATCH: &str =
    "authenticated worker identity does not match the selected runtime";

pub(super) fn decode_worker_payload<T: serde::de::DeserializeOwned>(
    request: &RequestEnvelope,
) -> std::result::Result<T, ErrorBody> {
    from_value(request.payload.clone()).map_err(|error| ErrorBody::invalid(error.to_string()))
}

pub(super) fn protocol_error(error: ProtocolError) -> ErrorBody {
    ErrorBody::internal(error.to_string())
}

pub(super) fn internal_error(error: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::internal(error.to_string())
}

pub(super) fn termination_process_error(error: anyhow::Error) -> ErrorBody {
    if error.is::<termination_timing::CapabilitySettlementDeferred>() {
        return ErrorBody::new(
            crate::protocol::error_code::CLEANUP_DEFERRED,
            error.to_string(),
            false,
        );
    }
    if error.is::<crate::process::DestructiveSessionUnavailable>() {
        return ErrorBody::new(
            crate::protocol::error_code::DESTRUCTIVE_SESSION_UNAVAILABLE,
            error.to_string(),
            false,
        );
    }
    internal_error(error)
}

#[cfg(test)]
pub(super) fn signal_number(name: &str) -> Option<i32> {
    process::parse_signal(name).ok()
}

pub(super) fn validate_socket_path(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        if path.as_os_str().as_bytes().len() >= UNIX_SOCKET_PATH_MAX_BYTES {
            bail!("worker socket path is too long: {}", path.display());
        }
    }
    Ok(())
}

pub(super) async fn worker_operation_with_timeout<F, T>(timeout: Duration, operation: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::time::timeout(timeout, operation)
        .await
        .context(WORKER_OPERATION_TIMEOUT_CONTEXT)?
}

pub(super) async fn authenticated_stream_inner_with_server_verification(
    record: &RuntimeRecord,
    controller_generation: u64,
    verification: WorkerServerVerification,
) -> std::result::Result<(WorkerConnection, WorkerHello), WorkerRpcError> {
    let mut stream = UnixStream::connect(&record.socket_path)
        .await
        .map_err(WorkerRpcError::transport)?;
    verify_worker_server(&stream, record, verification)?;
    let request = RequestEnvelope::new(
        Uuid::new_v4().to_string(),
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: record.auth_token.clone(),
            controller_generation,
        })
        .map_err(WorkerRpcError::protocol)?,
    );
    write_frame(&mut stream, &request)
        .await
        .map_err(WorkerRpcError::from_protocol)?;
    let response: ResponseEnvelope = read_frame(&mut stream)
        .await
        .map_err(WorkerRpcError::from_protocol)?;
    validate_response(&request, &response).map_err(WorkerRpcError::from_protocol)?;
    if !response.ok {
        return Err(response
            .error
            .map(WorkerRpcError::remote)
            .unwrap_or_else(|| WorkerRpcError::protocol(WORKER_AUTH_FAILURE_CONTEXT)));
    }
    let authenticated: WorkerHello = response
        .decode_payload()
        .map_err(WorkerRpcError::from_protocol)?;
    if !authenticated.runtime.has_same_process_identity(record)
        || authenticated.runtime.controller_generation != controller_generation
    {
        return Err(WorkerRpcError::identity_mismatch(
            WORKER_AUTH_IDENTITY_MISMATCH,
        ));
    }
    Ok((WorkerConnection::open(stream).await?, authenticated))
}

pub async fn authenticated_stream(
    record: &RuntimeRecord,
    controller_generation: u64,
) -> std::result::Result<WorkerConnection, WorkerRpcError> {
    let (stream, _) = worker_rpc_with_timeout(
        WORKER_AUTH_TIMEOUT,
        WORKER_AUTH_TIMEOUT_CONTEXT,
        authenticated_stream_inner(record, controller_generation),
    )
    .await?;
    Ok(stream)
}

pub async fn authenticated_runtime(
    record: &RuntimeRecord,
    controller_generation: u64,
) -> std::result::Result<RuntimeRecord, WorkerRpcError> {
    let (_, authenticated) = worker_rpc_with_timeout(
        WORKER_AUTH_TIMEOUT,
        WORKER_AUTH_TIMEOUT_CONTEXT,
        authenticated_stream_inner(record, controller_generation),
    )
    .await?;
    Ok(authenticated.runtime)
}

pub async fn exchange_worker_request(
    stream: &mut WorkerConnection,
    request: &RequestEnvelope,
) -> std::result::Result<ResponseEnvelope, WorkerRpcError> {
    stream.exchange(request).await
}

pub fn load_exit(paths: &Paths, terminal_id: Uuid) -> Option<ExitRecord> {
    read_json(&paths.exit_record(terminal_id)).ok()
}

pub fn load_snapshot(paths: &Paths, terminal_id: Uuid) -> Option<TerminalSnapshot> {
    read_snapshot(&paths.snapshot(terminal_id)).ok()
}
