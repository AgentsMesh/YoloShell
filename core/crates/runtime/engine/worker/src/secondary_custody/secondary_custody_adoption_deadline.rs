use anyhow::{Context, Result};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

pub(crate) const MUTATION_GATE_TIMEOUT_CONTEXT: &str = "PTY successor mutation gate timed out";

#[derive(Clone, Copy)]
pub(crate) struct WorkerAdoptionDeadline {
    expires_at: Instant,
}

impl WorkerAdoptionDeadline {
    pub(crate) fn production() -> Self {
        Self::from_timeout(crate::session_timing::WORKER_READY_TIMEOUT)
    }

    pub(crate) fn from_timeout(timeout: Duration) -> Self {
        Self {
            expires_at: Instant::now() + timeout,
        }
    }

    pub(crate) async fn wait<Output>(
        self,
        future: impl Future<Output = Output>,
        timeout_context: &'static str,
    ) -> Result<Output> {
        tokio::time::timeout_at(self.expires_at, future)
            .await
            .context(timeout_context)
    }

    pub(crate) async fn admit_system_mutation(
        self,
        state: &Arc<super::WorkerState>,
    ) -> Result<super::SystemMutationPermit> {
        self.wait(state.admit_system_mutation(), MUTATION_GATE_TIMEOUT_CONTEXT)
            .await
    }
}
