#[cfg(test)]
use super::constants::{
    CODEX_PROTOCOL_REJECTION_LOG, CODEX_REJECTION_LOG, CODEX_REQUEST_TIMEOUT,
    CODEX_REQUEST_TIMEOUT_ERROR,
};
use super::constants::{CODEX_RESPONSE_TIMEOUT, CODEX_RESPONSE_TIMEOUT_ERROR};
#[cfg(test)]
use super::outcome::CodexHookHandleOutcome;
#[cfg(test)]
use super::protocol::read_envelope;
use super::protocol::{CodexHookAck, write_ack};
#[cfg(test)]
use super::runtime::CodexHookRuntime;
#[cfg(test)]
use crate::agent::hooks::ApplyDisposition;
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result};
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
#[path = "handler_handoff.rs"]
mod handoff;
#[cfg(test)]
use handoff::BlockingIngest;

#[cfg(test)]
pub(super) async fn handle(
    stream: LocalUnixStream,
    runtime: Arc<CodexHookRuntime>,
) -> Result<CodexHookHandleOutcome> {
    handle_custodied(
        stream,
        runtime,
        super::blocking_custody::BlockingSettlementCustody::default(),
    )
    .await
}

#[cfg(test)]
pub(super) async fn handle_custodied(
    mut stream: LocalUnixStream,
    runtime: Arc<CodexHookRuntime>,
    settlements: super::blocking_custody::BlockingSettlementCustody,
) -> Result<CodexHookHandleOutcome> {
    let peer = super::peer_identity::verify(&stream)?;
    let event = match tokio::time::timeout(CODEX_REQUEST_TIMEOUT, read_envelope(&mut stream)).await
    {
        Ok(Ok(event)) => event,
        Ok(Err(error)) => {
            tracing::warn!(detail = %error, message = CODEX_PROTOCOL_REJECTION_LOG);
            let response = write_response(&mut stream, &CodexHookAck::protocol_rejection()).await;
            return Ok(CodexHookHandleOutcome::new(None, response));
        }
        Err(error) => {
            let error = anyhow::Error::new(error).context(CODEX_REQUEST_TIMEOUT_ERROR);
            tracing::warn!(detail = %error, message = CODEX_PROTOCOL_REJECTION_LOG);
            let response = write_response(&mut stream, &CodexHookAck::protocol_rejection()).await;
            return Ok(CodexHookHandleOutcome::new(None, response));
        }
    };
    handle_event(&mut stream, runtime, event, peer.pid, settlements).await
}

#[cfg(test)]
pub(super) async fn handle_event(
    stream: &mut LocalUnixStream,
    runtime: Arc<CodexHookRuntime>,
    event: crate::agent::hooks::HookEnvelope,
    peer_pid: u32,
    settlements: super::blocking_custody::BlockingSettlementCustody,
) -> Result<CodexHookHandleOutcome> {
    let handoff = handoff::begin(Arc::clone(&runtime), event.clone(), peer_pid, settlements).await;
    let (ack, disposition) = match &handoff.outcome {
        BlockingIngest::Completed(Ok(disposition)) => {
            let replay = *disposition == ApplyDisposition::Replay;
            (CodexHookAck::accepted(&event, replay), Some(*disposition))
        }
        BlockingIngest::Completed(Err(error)) => {
            tracing::warn!(detail = %error, message = CODEX_REJECTION_LOG);
            (CodexHookAck::rejected(Some(&event), error.code()), None)
        }
        BlockingIngest::Failed(error) => {
            tracing::warn!(detail = %error, message = CODEX_REJECTION_LOG);
            (CodexHookAck::protocol_rejection(), None)
        }
    };
    let response = write_response(stream, &ack).await;
    #[cfg(test)]
    runtime.wait_for_response_gate_for_test().await;
    let settlement = handoff.settle().await;
    let response = settlement.and(response);
    Ok(CodexHookHandleOutcome::new(disposition, response))
}

pub(super) async fn write_response(stream: &mut LocalUnixStream, ack: &CodexHookAck) -> Result<()> {
    tokio::time::timeout(CODEX_RESPONSE_TIMEOUT, write_ack(stream, ack))
        .await
        .context(CODEX_RESPONSE_TIMEOUT_ERROR)??;
    Ok(())
}

pub(super) async fn write_generation_response(
    stream: &mut LocalUnixStream,
    generation: &super::cancellation::IngressCancellation,
    ack: &CodexHookAck,
) -> Result<()> {
    let _fence = generation
        .try_fence()
        .context(super::constants::CODEX_INGRESS_GENERATION_REVOKED_ERROR)?;
    write_response(stream, ack).await
}
