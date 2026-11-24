use super::super::secondary_custody_export::SuccessorCustodyExportLease;
use super::super::{WorkerCustodyLease, WorkerState};
use anyhow::{Context, Result};
use std::sync::Arc;

const ROLLBACK_CONTEXT: &str = "release rejected PTY successor descriptors before activation";
const ROLLBACK_TIMEOUT_CONTEXT: &str = "PTY successor pre-activation rollback timed out";

pub(crate) async fn fail_post_ready<T>(
    _state: &Arc<WorkerState>,
    error: anyhow::Error,
) -> Result<T> {
    Err(error)
}

pub(crate) async fn rollback_post_ready<T>(
    state: &Arc<WorkerState>,
    lease: WorkerCustodyLease,
    export: &mut Option<SuccessorCustodyExportLease>,
    mut error: anyhow::Error,
) -> Result<T> {
    let rollback = tokio::time::timeout(
        crate::session_timing::WORKER_READY_TIMEOUT,
        lease.rollback_before_activation(),
    )
    .await
    .context(ROLLBACK_TIMEOUT_CONTEXT)
    .and_then(|result| result);
    match rollback {
        Ok(_) => {
            if let Some(mut export) = export.take() {
                export.resolve();
            }
        }
        Err(rollback) => {
            drop(export.take());
            error = error.context(format!("{ROLLBACK_CONTEXT}: {rollback:#}"));
        }
    }
    fail_post_ready(state, error).await
}
