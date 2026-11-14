use super::super::router::CodexRouteSubscription;
use crate::worker::WorkerState;
use anyhow::Result;
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

const PROJECTION_INVALIDATION_FAILED: &str = "Codex projection authority invalidation failed";
const PROJECTION_TASK_JOIN_FAILED: &str = "Codex projection task join failed";
const PROJECTION_TASK_PANICKED: &str = "Codex projection task panicked";
const PROJECTION_TASK_STOPPED: &str = "Codex projection task stopped unexpectedly";
#[cfg(test)]
const NO_ROUTE_RECEIVERS: usize = 0;

pub(super) async fn run(
    state: Arc<WorkerState>,
    subscription: Option<CodexRouteSubscription>,
) -> Result<()> {
    let operation = super::run_once(Arc::clone(&state), subscription);
    let outcome = AssertUnwindSafe(operation).catch_unwind().await;
    if matches!(&outcome, Ok(Ok(()))) && state.response_shutdown_is_committed() {
        // Runtime settlement owns the final invalidation and manifest-revocation proof.
        return Ok(());
    }
    let primary = caught_failure(outcome);
    combine(primary, super::publication::invalidate(&state).await)
}

pub(in crate::worker) async fn complete(
    state: Arc<WorkerState>,
    task: tokio::task::JoinHandle<Result<()>>,
) -> Result<()> {
    let primary = primary_failure(task.await);
    combine(primary, super::publication::invalidate(&state).await)
}

fn caught_failure(
    outcome: std::result::Result<Result<()>, Box<dyn std::any::Any + Send>>,
) -> anyhow::Error {
    match outcome {
        Ok(outcome) => primary_failure(Ok(outcome)),
        Err(payload) => anyhow::anyhow!(crate::worker::supervision::panic_detail(payload))
            .context(PROJECTION_TASK_PANICKED),
    }
}

fn primary_failure(
    outcome: std::result::Result<Result<()>, tokio::task::JoinError>,
) -> anyhow::Error {
    match outcome {
        Ok(Ok(())) => anyhow::anyhow!(PROJECTION_TASK_STOPPED),
        Ok(Err(error)) => error,
        Err(error) => anyhow::Error::new(error).context(PROJECTION_TASK_JOIN_FAILED),
    }
}

fn combine(primary: anyhow::Error, invalidation: Result<bool>) -> Result<()> {
    match invalidation {
        Ok(_) => Err(primary),
        Err(invalidation) => Err(primary.context(format!(
            "{PROJECTION_INVALIDATION_FAILED}: {invalidation:#}"
        ))),
    }
}

#[cfg(test)]
pub(in crate::worker) struct ProjectionRoundProbe {
    routes: tokio::sync::broadcast::Sender<super::super::router::CodexCommittedRoute>,
}

#[cfg(test)]
impl ProjectionRoundProbe {
    pub(in crate::worker) fn run_once_is_attached(&self) -> bool {
        self.routes.receiver_count() != NO_ROUTE_RECEIVERS
    }
}

#[cfg(test)]
pub(in crate::worker) async fn run_idle_for_test(
    state: Arc<WorkerState>,
    entered: tokio::sync::oneshot::Sender<ProjectionRoundProbe>,
) -> Result<()> {
    let (routes, receiver) =
        tokio::sync::broadcast::channel(super::super::constants::CODEX_PROJECTION_CHANNEL_CAPACITY);
    let probe = ProjectionRoundProbe {
        routes: routes.clone(),
    };
    let subscription = CodexRouteSubscription {
        snapshot: Vec::new(),
        receiver,
    };
    let _ = entered.send(probe);
    let _routes = routes;
    run(state, Some(subscription)).await
}

#[cfg(test)]
pub(in crate::worker) async fn supervise_closed_for_test(state: Arc<WorkerState>) -> Result<()> {
    let (routes, receiver) =
        tokio::sync::broadcast::channel(super::super::constants::CODEX_PROJECTION_CHANNEL_CAPACITY);
    drop(routes);
    super::supervise(
        state,
        CodexRouteSubscription {
            snapshot: Vec::new(),
            receiver,
        },
    )
    .await
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_round_inline_tests.rs"]
mod tests;
