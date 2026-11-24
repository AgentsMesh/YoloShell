use super::constants::{
    CODEX_PROTOCOL_REJECTION_LOG, CODEX_REQUEST_TIMEOUT, CODEX_REQUEST_TIMEOUT_ERROR,
};
use super::hook_context::CodexHookContext;
use super::outcome::CodexHookHandleOutcome;
use super::protocol::{CodexHookAck, CodexIngressRequest, read_request};
use super::router::CodexSessionRouter;
use crate::local_unix::LocalUnixStream;
use anyhow::Result;
use std::sync::Arc;

#[path = "routed_handler_event.rs"]
mod event;
#[path = "routed_handler_native.rs"]
mod native;

pub(super) async fn handle(
    mut stream: LocalUnixStream,
    router: Arc<CodexSessionRouter>,
    transaction: Arc<CodexHookContext>,
    settlements: super::blocking_custody::BlockingSettlementCustody,
    generation: super::cancellation::IngressCancellation,
) -> Result<CodexHookHandleOutcome> {
    let peer = super::peer_identity::verify(&stream)?;
    let request = match tokio::time::timeout(CODEX_REQUEST_TIMEOUT, read_request(&mut stream)).await
    {
        Ok(Ok(request)) => request,
        Ok(Err(error)) => return reject_protocol(&mut stream, &generation, error).await,
        Err(error) => {
            let error = anyhow::Error::new(error).context(CODEX_REQUEST_TIMEOUT_ERROR);
            return reject_protocol(&mut stream, &generation, error).await;
        }
    };
    match request {
        CodexIngressRequest::Native(request) => {
            native::handle(
                &mut stream,
                transaction.as_ref(),
                request,
                peer,
                settlements,
                generation,
            )
            .await
        }
        CodexIngressRequest::Event(envelope) => {
            event::handle(
                &mut stream,
                router.as_ref(),
                envelope,
                peer,
                settlements,
                generation,
            )
            .await
        }
    }
}

async fn reject_protocol(
    stream: &mut LocalUnixStream,
    generation: &super::cancellation::IngressCancellation,
    error: anyhow::Error,
) -> Result<CodexHookHandleOutcome> {
    tracing::warn!(detail = %error, message = CODEX_PROTOCOL_REJECTION_LOG);
    let response = super::handler::write_generation_response(
        stream,
        generation,
        &CodexHookAck::protocol_rejection(),
    )
    .await;
    Ok(CodexHookHandleOutcome::new(None, response))
}
