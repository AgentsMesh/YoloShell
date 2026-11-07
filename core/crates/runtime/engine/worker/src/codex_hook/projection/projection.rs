#[path = "projection_binding.rs"]
mod binding;
#[path = "projection_cursor.rs"]
mod cursor;
#[path = "projection_control.rs"]
mod control;
pub(in crate::worker::codex_hook) use control::require_current_control_projection;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/hook/control_source_tests.rs"]
mod control_source_tests;
#[path = "projection_forwarder.rs"]
mod forwarder;
#[path = "projection_publication.rs"]
mod publication;
#[path = "projection_round.rs"]
mod round;
#[path = "projection_route.rs"]
mod route;
#[path = "projection_shutdown.rs"]
mod shutdown;
#[path = "projection_snapshot.rs"]
mod snapshot;

#[cfg(test)]
pub(in crate::worker) use round::{
    ProjectionRoundProbe, complete as complete_task_for_test, run_idle_for_test,
    supervise_closed_for_test,
};

use super::publication::CodexAuthorityView;
use super::router::CodexRouteSubscription;
use crate::worker::{WorkerState, supervision};
use anyhow::{Context, Result, bail};
use cursor::{CursorDecision, ProjectionCursor, ProjectionPhase};
use forwarder::{ProjectionForwarders, ProjectionInput};
use std::sync::Arc;
use tokio::task::JoinSet;

pub(in crate::worker) fn spawn(
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    state: Arc<WorkerState>,
) -> Result<()> {
    let subscription = state.codex_hook_ingress.subscribe_projection_routes()?;
    let completion = state.codex_hook_ingress.track_projection()?;
    supervision::spawn_task(
        tasks,
        supervision::CODEX_AUTHORITY_PROJECTION_TASK,
        async move {
            let outcome = supervise(state, subscription).await;
            completion.finish(&outcome);
            outcome
        },
    );
    Ok(())
}

pub(in crate::worker) async fn invalidate_for_shutdown(
    state: &WorkerState,
    disposition: crate::worker::AgentProjectionShutdown,
) -> Result<bool> {
    shutdown::settle(state, disposition).await
}

async fn supervise(state: Arc<WorkerState>, subscription: CodexRouteSubscription) -> Result<()> {
    let mut initial = Some(subscription);
    let mut failures = u32::default();
    loop {
        let error = match round::run(Arc::clone(&state), initial.take()).await {
            Ok(()) => return Ok(()),
            Err(error) => error,
        };
        if state.response_shutdown_is_committed() {
            return Err(error);
        }
        failures = failures.saturating_add(1);
        let failure = supervision::TaskFailure::message(
            supervision::CODEX_AUTHORITY_PROJECTION_TASK,
            supervision::error_detail(error),
        );
        let delay =
            supervision::report_restart(&failure, supervision::RestartPolicy::default(), failures);
        tokio::time::sleep(delay).await;
    }
}

#[path = "projection_driver.rs"]
mod driver;
use driver::run_once;

async fn consume(
    state: &WorkerState,
    cursor: &mut ProjectionCursor,
    input: ProjectionInput,
) -> Result<()> {
    let (route, view, closed) = match input {
        ProjectionInput::View { route, view } => (route, Some(view), false),
        ProjectionInput::Closed { route } => (route, None, true),
    };
    if closed && cursor.preserves_closed(&route) {
        return Ok(());
    }
    let version = match view.as_ref() {
        Some(CodexAuthorityView::Active(lifecycle) | CodexAuthorityView::Exhausted(lifecycle)) => {
            ProjectionCursor::lifecycle_version(&route, lifecycle, lifecycle.authority_revision())
        }
        _ => cursor.terminal_version(&route),
    };
    let phase = match view.as_ref() {
        Some(CodexAuthorityView::Active(lifecycle)) if binding::confirmed_exit(lifecycle) => {
            ProjectionPhase::ConfirmedExit
        }
        Some(CodexAuthorityView::Active(_)) => ProjectionPhase::Active,
        Some(CodexAuthorityView::Exhausted(_)) => ProjectionPhase::Exhausted,
        _ => ProjectionPhase::Terminal,
    };
    match cursor.classify(version, phase) {
        CursorDecision::Stale | CursorDecision::Replay => return Ok(()),
        CursorDecision::Conflict => {
            publication::invalidate(state).await?;
            bail!(super::constants::CODEX_PROJECTION_VERSION_CONFLICT);
        }
        CursorDecision::Apply => {}
    }
    let projected = match view.as_ref() {
        Some(CodexAuthorityView::Active(lifecycle)) => {
            publication::publish_lifecycle(
                state,
                &route,
                lifecycle,
                lifecycle.authority_revision(),
                binding::ProjectionMode::Active,
            )
            .await?
        }
        Some(CodexAuthorityView::Exhausted(lifecycle)) => {
            publication::publish_lifecycle(
                state,
                &route,
                lifecycle,
                lifecycle.authority_revision(),
                binding::ProjectionMode::Exhausted,
            )
            .await?
        }
        _ => false,
    };
    if !projected {
        publication::invalidate(state).await?;
    }
    cursor.advance(version, phase);
    Ok(())
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_lifecycle_adversarial_tests.rs"]
mod lifecycle_adversarial_tests;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/projection/projection_lifecycle_test_storage.rs"]
pub(in crate::worker::codex_hook) mod lifecycle_test_storage;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_state_tests.rs"]
mod state_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_stop_candidate_tests.rs"]
mod stop_candidate_tests;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/projection/projection_test_support.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_tests.rs"]
mod tests;
