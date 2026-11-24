use super::super::constants::{
    CODEX_ACK_IDENTITY, CODEX_ACK_PROTOCOL, CODEX_ACK_REVOKED, CODEX_REJECTION_LOG,
};
use super::super::outcome::CodexHookHandleOutcome;
use super::super::protocol::CodexHookAck;
use super::super::router::{CodexRouterError, CodexSessionRouter};
use crate::agent::hooks::HookEnvelope;
use crate::local_unix::LocalUnixStream;
use anyhow::Result;

pub(super) async fn handle(
    stream: &mut LocalUnixStream,
    router: &CodexSessionRouter,
    event: HookEnvelope,
    peer: crate::process::ProcessIdentity,
    settlements: super::super::blocking_custody::BlockingSettlementCustody,
    generation: super::super::cancellation::IngressCancellation,
) -> Result<CodexHookHandleOutcome> {
    let lease = match router.lease_committed(&event.identity) {
        Ok(lease) => lease,
        Err(error) => return reject(stream, &event, &generation, error).await,
    };
    let runtime = std::sync::Arc::clone(lease.entry().runtime());
    let producer = super::super::runtime::CodexLiveProducer::new(
        peer,
        lease.entry().admission().clone(),
        generation,
    );
    let outcome =
        super::super::live_handler::handle(stream, runtime, event, producer, settlements).await;
    let settlement = lease.settle().map_err(anyhow::Error::new);
    settlement?;
    outcome
}

async fn reject(
    stream: &mut LocalUnixStream,
    event: &HookEnvelope,
    generation: &super::super::cancellation::IngressCancellation,
    error: CodexRouterError,
) -> Result<CodexHookHandleOutcome> {
    tracing::warn!(detail = %error, message = CODEX_REJECTION_LOG);
    let ack = CodexHookAck::rejected(Some(event), rejection_code(error));
    let response = super::super::handler::write_generation_response(stream, generation, &ack).await;
    Ok(CodexHookHandleOutcome::new(None, response))
}

fn rejection_code(error: CodexRouterError) -> &'static str {
    match error {
        CodexRouterError::Scope => CODEX_ACK_IDENTITY,
        CodexRouterError::Runtime
        | CodexRouterError::Admission
        | CodexRouterError::Conflict
        | CodexRouterError::Capacity => CODEX_ACK_REVOKED,
        CodexRouterError::Lock => CODEX_ACK_PROTOCOL,
    }
}
