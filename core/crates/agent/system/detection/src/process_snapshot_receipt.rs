use super::limits::{MAX_PROCESS_SNAPSHOT_ERROR_BYTES, PROCESS_SNAPSHOT_SCHEMA_VERSION};
use super::wire::{ProcessSnapshotEnvelope, ProcessSnapshotWireOutcome};
use super::{ProcessSnapshotOutcome, ProcessSnapshotReceipt};
use crate::process::ProcessIdentity;
use anyhow::{Context, Result};

const DECODE_TASK_CONTEXT: &str = "join bounded process snapshot decode task";

pub(super) async fn decode(
    payload: Vec<u8>,
    root: ProcessIdentity,
    environment_keys: Vec<String>,
) -> Result<ProcessSnapshotReceipt> {
    run_in_background(move || decode_bounded(&payload, root, &environment_keys)).await
}

async fn run_in_background<Output>(
    work: impl FnOnce() -> Result<Output> + Send + 'static,
) -> Result<Output>
where
    Output: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .context(DECODE_TASK_CONTEXT)?
}

fn decode_bounded(
    payload: &[u8],
    root: ProcessIdentity,
    environment_keys: &[String],
) -> Result<ProcessSnapshotReceipt> {
    let envelope: ProcessSnapshotEnvelope = super::decode::decode_strict(payload)?;
    anyhow::ensure!(
        envelope.schema_version == PROCESS_SNAPSHOT_SCHEMA_VERSION
            && envelope.root_pid == root.pid
            && envelope.root_start_time == root.start_time
            && super::validation::valid_platform_pid(root.pid)
            && root.start_time != 0,
        super::super::constants::process_error::SNAPSHOT_INVALID
    );
    anyhow::ensure!(
        envelope.started_at_ms > 0 && envelope.completed_at_ms >= envelope.started_at_ms,
        super::super::constants::process_error::SNAPSHOT_INVALID
    );
    let outcome = match envelope.outcome {
        ProcessSnapshotWireOutcome::Stable { processes } => {
            ProcessSnapshotOutcome::Stable(super::validation::decode_processes(
                processes,
                environment_keys,
            )?)
        }
        ProcessSnapshotWireOutcome::RootChanged => ProcessSnapshotOutcome::RootChanged,
        ProcessSnapshotWireOutcome::Failed { detail } => {
            anyhow::ensure!(
                !detail.is_empty() && detail.len() <= MAX_PROCESS_SNAPSHOT_ERROR_BYTES,
                super::super::constants::process_error::SNAPSHOT_INVALID
            );
            ProcessSnapshotOutcome::Failed(detail)
        }
    };
    Ok(ProcessSnapshotReceipt {
        outcome,
        started_at_ms: envelope.started_at_ms,
        completed_at_ms: envelope.completed_at_ms,
    })
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/process_snapshot_receipt_inline_tests.rs"]
mod tests;
