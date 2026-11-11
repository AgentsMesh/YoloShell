use super::{
    CodexHookHandleOutcome, CodexNativeAck, CodexNativeAckCode, CodexNativeRequest,
    LocalUnixStream, NativeCodexHandoff, Result,
};
use crate::worker::codex_hook::live_handoff::BlockingLiveIngest;
use crate::worker::codex_hook::runtime::CodexLiveProducer;
use std::sync::Arc;

pub(super) async fn handle(
    stream: &mut LocalUnixStream,
    request: CodexNativeRequest,
    mut route: NativeCodexHandoff,
    peer: crate::process::ProcessIdentity,
    settlements: crate::worker::codex_hook::blocking_custody::BlockingSettlementCustody,
    generation: crate::worker::codex_hook::cancellation::IngressCancellation,
) -> Result<CodexHookHandleOutcome> {
    let runtime = Arc::clone(route.lease.entry().runtime());
    let replay = route.lease.is_replay() || !route.first_event;
    let producer = CodexLiveProducer::new(
        peer,
        route.lease.entry().admission().clone(),
        generation.clone(),
    );
    let staged = Arc::clone(&runtime);
    let event = route.event.clone();
    let settlement = settlements.admit();
    route.event = tokio::task::spawn_blocking(move || {
        let _settlement = settlement;
        staged.persist_native_event(event)
    })
    .await??;
    let handoff =
        crate::worker::codex_hook::live_handoff::begin(runtime, route.event, producer, settlements)
            .await;
    let (ack, disposition) = match &handoff.outcome {
        BlockingLiveIngest::Completed(Ok(ingest)) if ingest.rejection.is_none() => (
            CodexNativeAck::accepted(
                &request,
                replay
                    && ingest.publication.disposition
                        == crate::agent::hooks::ApplyDisposition::Replay,
            ),
            Some(ingest.publication.disposition),
        ),
        BlockingLiveIngest::CommitIndeterminate(_) => {
            let result = handoff.settle(false).await;
            route.lease.settle().map_err(anyhow::Error::new)?;
            return Ok(CodexHookHandleOutcome::new(
                None,
                result.and(Err(anyhow::anyhow!(
                    crate::worker::codex_hook::constants::CODEX_COMMIT_INDETERMINATE_ERROR
                ))),
            ));
        }
        _ => (
            CodexNativeAck::rejected(&request, CodexNativeAckCode::AuthorityRevoked),
            None,
        ),
    };
    route.lease.settle().map_err(anyhow::Error::new)?;
    let response = super::write_response(stream, &generation, &ack).await;
    let settled = handoff.settle(response.is_ok()).await;
    Ok(CodexHookHandleOutcome::new(
        disposition,
        settled.and(response),
    ))
}
