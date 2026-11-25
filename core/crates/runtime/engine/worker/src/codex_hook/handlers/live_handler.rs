use super::constants::CODEX_REJECTION_LOG;
use super::error::CodexHookRejection;
use super::live_handoff::BlockingLiveIngest;
use super::outcome::CodexHookHandleOutcome;
use super::protocol::CodexHookAck;
use super::runtime::{CodexHookRuntime, CodexLiveProducer};
use crate::local_unix::LocalUnixStream;
use anyhow::Result;
use anyhow::anyhow;
use std::sync::Arc;

pub(super) async fn handle(
    stream: &mut LocalUnixStream,
    runtime: Arc<CodexHookRuntime>,
    event: crate::agent::hooks::HookEnvelope,
    producer: CodexLiveProducer,
    settlements: super::blocking_custody::BlockingSettlementCustody,
) -> Result<CodexHookHandleOutcome> {
    let generation = producer.generation();
    let handoff =
        super::live_handoff::begin(Arc::clone(&runtime), event.clone(), producer, settlements)
            .await;
    if let BlockingLiveIngest::CommitIndeterminate(error) = &handoff.outcome {
        tracing::warn!(detail = %error.rejection(), message = CODEX_REJECTION_LOG);
        let response = handoff.settle(false).await.and(Err(anyhow!(
            super::constants::CODEX_COMMIT_INDETERMINATE_ERROR
        )));
        return Ok(CodexHookHandleOutcome::new(None, response));
    }
    let (ack, disposition) = match &handoff.outcome {
        BlockingLiveIngest::Completed(Ok(ingest)) => match ingest.rejection {
            Some(violation) => (
                CodexHookAck::rejected(
                    Some(&ingest.event),
                    CodexHookRejection::Violation(violation).code(),
                ),
                None,
            ),
            None => {
                let replay =
                    ingest.publication.disposition == crate::agent::hooks::ApplyDisposition::Replay;
                (
                    CodexHookAck::accepted(&ingest.event, replay),
                    Some(ingest.publication.disposition),
                )
            }
        },
        BlockingLiveIngest::Completed(Err(error)) => {
            tracing::warn!(detail = %error.rejection(), message = CODEX_REJECTION_LOG);
            (
                CodexHookAck::rejected(Some(error.event(&event)), error.code()),
                None,
            )
        }
        BlockingLiveIngest::Failed(error) => {
            tracing::warn!(detail = %error, message = CODEX_REJECTION_LOG);
            (CodexHookAck::protocol_rejection(), None)
        }
        BlockingLiveIngest::CommitIndeterminate(_) => unreachable!(),
    };
    #[cfg(test)]
    runtime.wait_for_ack_write_gate_for_test().await;
    let response = super::handler::write_generation_response(stream, &generation, &ack).await;
    #[cfg(test)]
    runtime.wait_for_response_gate_for_test().await;
    let settlement = handoff.settle(response.is_ok()).await;
    let response = settlement.and(response);
    Ok(CodexHookHandleOutcome::new(disposition, response))
}
