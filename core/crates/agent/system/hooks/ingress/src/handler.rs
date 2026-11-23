use super::constants::*;
use super::contract::{ACCEPTED, NOT_READY, HookRequest, HookResponse};
use super::host::Context;
use anyhow::{Context as _, Result};
use yolo_hooks_runtime::EventKind;
use yolo_local_unix::LocalUnixStream;

pub(crate) async fn handle(
    mut stream: LocalUnixStream,
    context: Context,
    runtime_active: bool,
) -> Result<()> {
    let peer_pid = super::peer::pid(&stream)?;
    let request = tokio::time::timeout(
        REQUEST_TIMEOUT,
        super::contract::read_frame::<_, HookRequest>(&mut stream),
    )
    .await
    .context("provider hook request timed out")??;
    let mut response = HookResponse::rejection(&request);
    if !runtime_active {
        if request.is_valid() && is_session_start(&context, &request) {
            response.code = NOT_READY.to_owned();
        }
    } else {
        match super::transaction::execute(&context, peer_pid, &request).await {
            Ok(super::transaction::Outcome::Applied(record)) => {
                response.accepted = true;
                response.code = ACCEPTED.to_owned();
                response.invocation_id = Some(record.invocation_id);
                response.revision = Some(record.revision);
            }
            Ok(super::transaction::Outcome::Ignored) => {
                response.accepted = true;
                response.code = ACCEPTED.to_owned();
            }
            Err(error) => {
                tracing::warn!(detail = %format!("{error:#}"), "rejected provider hook request")
            }
        }
    }
    tokio::time::timeout(
        RESPONSE_TIMEOUT,
        super::contract::write_frame(&mut stream, &response),
    )
    .await
    .context("provider hook response timed out")??;
    Ok(())
}

fn is_session_start(context: &Context, request: &HookRequest) -> bool {
    context
        .host
        .codec(&request.provider)
        .and_then(|codec| codec.decode(&request.payload).ok().flatten())
        .is_some_and(|event| matches!(event.event, EventKind::SessionStart { .. }))
}
