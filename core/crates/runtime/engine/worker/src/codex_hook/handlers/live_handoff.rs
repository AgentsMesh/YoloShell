use super::blocking_custody::BlockingSettlementCustody;
use super::constants::{CODEX_HANDLER_JOIN_ERROR, CODEX_REJECTION_LOG};
use super::runtime::{
    CodexHookRuntime, CodexLiveFailure, CodexLiveHandoffOutcome, CodexLiveIngest,
    CodexLiveProducer, CodexLiveSettlement,
};
use crate::agent::hooks::HookEnvelope;
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

pub(super) enum BlockingLiveIngest {
    Completed(Result<CodexLiveIngest, CodexLiveFailure>),
    CommitIndeterminate(CodexLiveFailure),
    Failed(anyhow::Error),
}

pub(super) struct BlockingLiveHandoff {
    pub(super) outcome: BlockingLiveIngest,
    settlement: std::sync::mpsc::Sender<CodexLiveSettlement>,
    task: tokio::task::JoinHandle<()>,
    runtime: Arc<CodexHookRuntime>,
}

pub(super) async fn begin(
    runtime: Arc<CodexHookRuntime>,
    event: HookEnvelope,
    producer: CodexLiveProducer,
    settlements: BlockingSettlementCustody,
) -> BlockingLiveHandoff {
    let (outcome_tx, outcome_rx) = tokio::sync::oneshot::channel();
    let (settlement, settled) = std::sync::mpsc::channel();
    let ingestion_runtime = Arc::clone(&runtime);
    let blocking_custody = settlements.admit();
    let task = tokio::task::spawn_blocking(move || {
        let _blocking_custody = blocking_custody;
        let result = catch_unwind(AssertUnwindSafe(|| {
            ingestion_runtime.ingest_live_with_handoff(
                event,
                producer,
                |outcome: Result<CodexLiveIngest, CodexLiveFailure>| {
                    #[cfg(test)]
                    ingestion_runtime.wait_for_handoff_gate_for_test();
                    let indeterminate = outcome
                        .as_ref()
                        .is_err_and(CodexLiveFailure::is_commit_indeterminate);
                    let requires_settlement = outcome.is_ok();
                    let outcome = match (indeterminate, outcome) {
                        (true, Err(error)) => BlockingLiveIngest::CommitIndeterminate(error),
                        (_, outcome) => BlockingLiveIngest::Completed(outcome),
                    };
                    if outcome_tx.send(outcome).is_err() {
                        return CodexLiveHandoffOutcome::Deferred;
                    }
                    if !requires_settlement {
                        return CodexLiveHandoffOutcome::Completed;
                    }
                    let Ok(settlement) = settled.recv() else {
                        return CodexLiveHandoffOutcome::Deferred;
                    };
                    match ingestion_runtime.complete_live_settlement(settlement) {
                        Ok(()) => CodexLiveHandoffOutcome::Completed,
                        Err(error) => {
                            let indeterminate =
                                crate::codex_hooks::is_outbox_commit_indeterminate(&error);
                            tracing::warn!(detail = %error, message = CODEX_REJECTION_LOG);
                            if indeterminate {
                                CodexLiveHandoffOutcome::CommitIndeterminate
                            } else {
                                CodexLiveHandoffOutcome::Deferred
                            }
                        }
                    }
                },
            );
        }));
        if let Err(payload) = result {
            ingestion_runtime.contain_authority_panic();
            std::mem::forget(payload);
        }
    });
    let outcome = match outcome_rx.await {
        Ok(outcome) => outcome,
        Err(error) => {
            runtime.contain_authority_panic();
            BlockingLiveIngest::Failed(anyhow::Error::new(error).context(CODEX_HANDLER_JOIN_ERROR))
        }
    };
    BlockingLiveHandoff {
        outcome,
        settlement,
        task,
        runtime,
    }
}

impl BlockingLiveHandoff {
    pub(super) async fn settle(self, acknowledged: bool) -> Result<()> {
        let Self {
            outcome,
            settlement,
            task,
            runtime,
        } = self;
        if acknowledged {
            if let BlockingLiveIngest::Completed(Ok(ingest)) = outcome {
                let _ = settlement.send(ingest.settlement);
            }
        }
        drop(settlement);
        match task.await {
            Ok(()) => Ok(()),
            Err(error) => {
                runtime.contain_authority_panic();
                Err(anyhow::Error::new(error).context(CODEX_HANDLER_JOIN_ERROR))
            }
        }
    }
}
