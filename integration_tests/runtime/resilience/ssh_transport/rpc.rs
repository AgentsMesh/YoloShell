use super::constants::{TERMINAL_OUTPUT_TIMEOUT, TRANSPORT_EVENT_TIMEOUT};
use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use yolo_runtime_rpc::RuntimeConnection;
use yolo_runtime::model::OutputChunk;
use yolo_runtime::protocol::{
    ErrorBody, EventEnvelope, RequestEnvelope, event, from_value, method, to_value, validate_response,
};
use yolo_runtime::worker::TerminalRef;
use yolo_runtime::{TerminalRuntimeStatus, TerminalView};

const REMOTE_RPC_ENCODE_CONTEXT: &str = "encode remote RPC request";
const REMOTE_RPC_READ_CONTEXT: &str = "read remote RPC response";
const REMOTE_RPC_VALIDATE_CONTEXT: &str = "validate remote RPC response";
const REMOTE_RPC_FAILURE_DETAIL: &str = "remote RPC returned an error";
const REMOTE_RPC_MISSING_ERROR_DETAIL: &str = "failed remote RPC omitted its error body";
const REMOTE_RPC_DECODE_CONTEXT: &str = "decode remote RPC response";

pub(super) enum CheckedResponse<R> {
    Success(R),
    Failure(ErrorBody),
}

pub(super) async fn request<P, R>(stream: &mut RuntimeConnection, method: &str, payload: &P) -> R
where
    P: Serialize,
    R: DeserializeOwned,
{
    request_at(stream, method, payload, method).await
}

pub(super) async fn request_at<P, R>(
    stream: &mut RuntimeConnection,
    method: &str,
    payload: &P,
    stage: &str,
) -> R
where
    P: Serialize,
    R: DeserializeOwned,
{
    request_checked(stream, method, payload)
        .await
        .unwrap_or_else(|error| panic!("remote RPC {stage} ({method}) failed: {error:#}"))
}

pub(super) async fn request_checked<P, R>(
    stream: &mut RuntimeConnection,
    method: &str,
    payload: &P,
) -> Result<R>
where
    P: Serialize,
    R: DeserializeOwned,
{
    match request_outcome(stream, method, payload).await? {
        CheckedResponse::Success(value) => Ok(value),
        CheckedResponse::Failure(error) => {
            Err(anyhow::anyhow!("{REMOTE_RPC_FAILURE_DETAIL}: {error:?}"))
        }
    }
}

pub(super) async fn request_outcome<P, R>(
    stream: &mut RuntimeConnection,
    method: &str,
    payload: &P,
) -> Result<CheckedResponse<R>>
where
    P: Serialize,
    R: DeserializeOwned,
{
    let id = uuid::Uuid::new_v4().to_string();
    let encoded = to_value(payload).context(REMOTE_RPC_ENCODE_CONTEXT)?;
    let request = RequestEnvelope::new(id, method, encoded);
    let response = stream.exchange(&request, yolo_remote::RemoteDeadline::operation().instant())
        .await.context(REMOTE_RPC_READ_CONTEXT)?;
    validate_response(&request, &response).context(REMOTE_RPC_VALIDATE_CONTEXT)?;
    if response.ok {
        return response
            .decode_payload()
            .map(CheckedResponse::Success)
            .context(REMOTE_RPC_DECODE_CONTEXT);
    }
    response
        .error
        .map(CheckedResponse::Failure)
        .context(REMOTE_RPC_MISSING_ERROR_DETAIL)
}

pub(super) async fn assert_transport_closed(stream: &mut RuntimeConnection) {
    let request = RequestEnvelope::new(uuid::Uuid::new_v4().to_string(), method::HOST_STATUS,
        to_value(&()).unwrap());
    let result = stream.exchange(&request, tokio::time::Instant::now() + TRANSPORT_EVENT_TIMEOUT).await;
    assert!(result.is_err(), "SSH runtime channel did not observe transport loss");
}

pub(super) async fn wait_for_terminal_marker(stream: &mut RuntimeConnection, marker: &[u8]) {
    let deadline = tokio::time::Instant::now() + TERMINAL_OUTPUT_TIMEOUT;
    let mut output = Vec::new();
    while !output.windows(marker.len()).any(|window| window == marker) {
        let event: EventEnvelope = stream.next_event(deadline).await
            .expect("terminal attachment closed before marker");
        if event.event == event::TERMINAL_OUTPUT {
            let chunk: OutputChunk = from_value(event.payload).unwrap();
            output.extend_from_slice(&chunk.data);
        }
    }
}

pub(super) async fn wait_for_running(
    stream: &mut RuntimeConnection,
    terminal_id: uuid::Uuid,
) -> TerminalView {
    let deadline = tokio::time::Instant::now() + super::constants::RUNTIME_STATUS_TIMEOUT;
    loop {
        let view: TerminalView =
            request(stream, method::GET_SESSION, &TerminalRef { terminal_id }).await;
        let status = view.runtime.as_ref().unwrap().status;
        if status == TerminalRuntimeStatus::Running {
            return view;
        }
        assert!(
            matches!(status, TerminalRuntimeStatus::Starting | TerminalRuntimeStatus::Unreachable),
            "remote runtime entered terminal state {status:?}"
        );
        assert!(
            tokio::time::Instant::now() < deadline,
            "remote runtime did not become reachable"
        );
        tokio::time::sleep(super::constants::RUNTIME_STATUS_POLL_INTERVAL).await;
    }
}
