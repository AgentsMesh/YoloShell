use super::super::hook_context::{CodexHookContext, NativeCodexHandoff};
use super::super::outcome::CodexHookHandleOutcome;
use crate::codex_hooks::{CodexNativeAck, CodexNativeAckCode, CodexNativeRequest};
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result};

#[path = "../native/native_handler_event.rs"]
mod event;

pub(super) async fn handle(
    stream: &mut LocalUnixStream,
    transaction: &CodexHookContext,
    request: CodexNativeRequest,
    peer: crate::process::ProcessIdentity,
    settlements: super::super::blocking_custody::BlockingSettlementCustody,
    generation: super::super::cancellation::IngressCancellation,
) -> Result<CodexHookHandleOutcome> {
    let _serialization = transaction.admit_native_callback().await?;
    let handoff = match transaction.execute_native(peer, request.clone()).await {
        Ok(handoff) => handoff,
        Err(error) => {
            tracing::warn!(detail = %format!("{error:#}"), "rejected Codex native hook request");
            let code = if error
                .downcast_ref::<super::super::hook_context::NativePreflightFailure>()
                .is_some()
            {
                CodexNativeAckCode::ProcessUnavailable
            } else {
                CodexNativeAckCode::AuthorityRevoked
            };
            let ack = CodexNativeAck::rejected(&request, code);
            return Ok(CodexHookHandleOutcome::new(
                None,
                write_response(stream, &generation, &ack).await,
            ));
        }
    };
    event::handle(stream, request, handoff, peer, settlements, generation).await
}

async fn write_response(
    stream: &mut LocalUnixStream,
    generation: &super::super::cancellation::IngressCancellation,
    ack: &CodexNativeAck,
) -> Result<()> {
    let _fence = generation
        .try_fence()
        .context(super::super::constants::CODEX_INGRESS_GENERATION_REVOKED_ERROR)?;
    tokio::time::timeout(
        super::super::constants::CODEX_RESPONSE_TIMEOUT,
        crate::codex_hooks::write_native_ack(stream, ack),
    )
    .await
    .context(super::super::constants::CODEX_RESPONSE_TIMEOUT_ERROR)?
}
