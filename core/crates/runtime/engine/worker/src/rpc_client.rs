use super::*;

pub(super) async fn authenticated_stream_inner(
    record: &RuntimeRecord,
    controller_generation: u64,
) -> std::result::Result<(WorkerConnection, WorkerHello), WorkerRpcError> {
    authenticated_stream_inner_with_server_verification(
        record,
        controller_generation,
        WorkerServerVerification::ExactRuntime,
    )
    .await
}

pub async fn authenticated_stream_with_hello(
    record: &RuntimeRecord,
    controller_generation: u64,
) -> std::result::Result<(WorkerConnection, WorkerHello), WorkerRpcError> {
    worker_rpc_with_timeout(
        WORKER_AUTH_TIMEOUT,
        WORKER_AUTH_TIMEOUT_CONTEXT,
        authenticated_stream_inner(record, controller_generation),
    )
    .await
}

pub async fn rpc<T: Serialize>(
    record: &RuntimeRecord,
    controller_generation: u64,
    method: &str,
    payload: &T,
) -> std::result::Result<ResponseEnvelope, WorkerRpcError> {
    let (response, _) = rpc_authenticated(record, controller_generation, method, payload).await?;
    Ok(response)
}

pub async fn rpc_authenticated<T: Serialize>(
    record: &RuntimeRecord,
    controller_generation: u64,
    method: &str,
    payload: &T,
) -> std::result::Result<(ResponseEnvelope, WorkerHello), WorkerRpcError> {
    rpc_authenticated_with_server_verification(
        record,
        controller_generation,
        method,
        payload,
        WorkerServerVerification::ExactRuntime,
    )
    .await
}

#[cfg(any(test, feature = "host-test-hooks"))]
#[doc(hidden)]
pub async fn rpc_authenticated_for_host_test<T: Serialize>(
    record: &RuntimeRecord,
    controller_generation: u64,
    method: &str,
    payload: &T,
    peer: crate::process::ProcessIdentity,
) -> std::result::Result<(ResponseEnvelope, WorkerHello), WorkerRpcError> {
    rpc_authenticated_with_server_verification(
        record,
        controller_generation,
        method,
        payload,
        WorkerServerVerification::HostAuthorizedTestPeer(peer),
    )
    .await
}

pub(super) async fn rpc_authenticated_with_server_verification<T: Serialize>(
    record: &RuntimeRecord,
    controller_generation: u64,
    method: &str,
    payload: &T,
    verification: WorkerServerVerification,
) -> std::result::Result<(ResponseEnvelope, WorkerHello), WorkerRpcError> {
    let (mut stream, authenticated) = worker_rpc_with_timeout(
        WORKER_AUTH_TIMEOUT,
        WORKER_AUTH_TIMEOUT_CONTEXT,
        authenticated_stream_inner_with_server_verification(
            record,
            controller_generation,
            verification,
        ),
    )
    .await?;
    let payload = to_value(payload).map_err(WorkerRpcError::protocol)?;
    let request = RequestEnvelope::new(Uuid::new_v4().to_string(), method, payload);
    let response = stream.exchange(&request).await?;
    Ok((response, authenticated))
}

#[cfg(any(test, feature = "host-test-hooks"))]
#[doc(hidden)]
pub async fn authenticated_stream_for_host_test(
    record: &RuntimeRecord,
    controller_generation: u64,
    peer: crate::process::ProcessIdentity,
) -> std::result::Result<WorkerConnection, WorkerRpcError> {
    let (stream, _) = worker_rpc_with_timeout(
        WORKER_AUTH_TIMEOUT,
        WORKER_AUTH_TIMEOUT_CONTEXT,
        authenticated_stream_inner_with_server_verification(
            record,
            controller_generation,
            WorkerServerVerification::HostAuthorizedTestPeer(peer),
        ),
    )
    .await?;
    Ok(stream)
}

#[cfg(any(test, feature = "host-test-hooks"))]
#[doc(hidden)]
pub async fn authenticated_stream_with_hello_for_host_test(
    record: &RuntimeRecord,
    controller_generation: u64,
    peer: crate::process::ProcessIdentity,
) -> std::result::Result<(WorkerConnection, WorkerHello), WorkerRpcError> {
    worker_rpc_with_timeout(
        WORKER_AUTH_TIMEOUT,
        WORKER_AUTH_TIMEOUT_CONTEXT,
        authenticated_stream_inner_with_server_verification(
            record,
            controller_generation,
            WorkerServerVerification::HostAuthorizedTestPeer(peer),
        ),
    )
    .await
}

#[derive(Clone, Copy)]
pub(super) enum WorkerServerVerification {
    ExactRuntime,
    #[cfg(any(test, feature = "host-test-hooks"))]
    HostAuthorizedTestPeer(crate::process::ProcessIdentity),
}

pub(super) fn verify_worker_server(
    stream: &UnixStream,
    record: &RuntimeRecord,
    verification: WorkerServerVerification,
) -> std::result::Result<(), WorkerRpcError> {
    match verification {
        WorkerServerVerification::ExactRuntime => local_transport::verify_server(stream, record),
        #[cfg(any(test, feature = "host-test-hooks"))]
        WorkerServerVerification::HostAuthorizedTestPeer(peer) => {
            local_transport::verify_server_identity(stream, peer)
        }
    }
}

pub(super) fn rpc_timeout(method: &str) -> Duration {
    if method == crate::protocol::method::TERMINATE_RUNTIME_EXACT {
        WORKER_TERMINATION_TIMEOUT
    } else {
        WORKER_OPERATION_TIMEOUT
    }
}
