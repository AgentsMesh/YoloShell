use super::super::blocking_custody::BlockingSettlementCustody;
use super::super::constants::CODEX_HANDLER_JOIN_ERROR;
use super::super::error::CodexHookRejection;
use super::super::runtime::CodexHookRuntime;
use crate::agent::hooks::{ApplyDisposition, HookEnvelope};
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

pub(super) enum BlockingIngest {
    Completed(std::result::Result<ApplyDisposition, CodexHookRejection>),
    Failed(anyhow::Error),
}

pub(super) struct BlockingHandoff {
    pub(super) outcome: BlockingIngest,
    settlement: std::sync::mpsc::Sender<()>,
    task: tokio::task::JoinHandle<()>,
    runtime: Arc<CodexHookRuntime>,
}

pub(super) async fn begin(
    runtime: Arc<CodexHookRuntime>,
    event: HookEnvelope,
    peer_pid: u32,
    settlements: BlockingSettlementCustody,
) -> BlockingHandoff {
    let (outcome_tx, outcome_rx) = tokio::sync::oneshot::channel();
    let (settlement, settled) = std::sync::mpsc::channel();
    let ingestion_runtime = Arc::clone(&runtime);
    let blocking_custody = settlements.admit();
    let task = tokio::task::spawn_blocking(move || {
        let _blocking_custody = blocking_custody;
        let result = catch_unwind(AssertUnwindSafe(|| {
            ingestion_runtime.ingest_with_handoff(event, peer_pid, |outcome| {
                #[cfg(test)]
                ingestion_runtime.wait_for_handoff_gate_for_test();
                let accepted = outcome.is_ok();
                let outcome = outcome.map(|publication| publication.disposition);
                outcome_tx.send(BlockingIngest::Completed(outcome)).is_ok()
                    && (!accepted || settled.recv().is_ok())
            });
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
            BlockingIngest::Failed(anyhow::Error::new(error).context(CODEX_HANDLER_JOIN_ERROR))
        }
    };
    BlockingHandoff {
        outcome,
        settlement,
        task,
        runtime,
    }
}

impl BlockingHandoff {
    pub(super) async fn settle(self) -> Result<()> {
        let _ = self.settlement.send(());
        match self.task.await {
            Ok(()) => Ok(()),
            Err(error) => {
                self.runtime.contain_authority_panic();
                Err(anyhow::Error::new(error).context(CODEX_HANDLER_JOIN_ERROR))
            }
        }
    }
}
