//! Turn wait for the atomic agent prompt. The caller submits through the
//! native control path; this helper evaluates a post-submission condition
//! against observations and never inspects screen text.

use std::time::Duration;
use tokio::sync::watch;
use tokio::time::Instant;
use yolo_hub_agent_protocol::{
    Activity, AgentLifecycleState, AgentPromptOutcome, AgentPromptRequest, AgentPromptResult,
    AgentPromptWait, AgentSnapshot, AgentWaitCondition, AgentWaitRequest, CommandOutcome,
    MAX_AGENT_WAIT_MS, ObservationCursor,
};
use yolo_hub_identity::{AgentTarget, HubId};
use yolo_hub_terminal_protocol::PublicError;

use crate::{AgentSnapshots, wait};

/// Submit through the origin's authenticated process, then observe the requested
/// turn disposition with the same receipt for local and relayed calls.
pub async fn submit(
    backend: &yolo_hub_local_backend::LocalBackend,
    request: &AgentPromptRequest,
    wake: watch::Receiver<u64>,
) -> Result<AgentPromptResult, PublicError> {
    let execution = backend.prompt_agent(request).await?;
    let (outcome, snapshot) = match execution.receipt.outcome() {
        CommandOutcome::Failed { .. } => (AgentPromptOutcome::Rejected, None),
        CommandOutcome::Unknown {} => (AgentPromptOutcome::Unknown, None),
        _ => match request.fields().wait {
            AgentPromptWait::None => (AgentPromptOutcome::Delivered, None),
            AgentPromptWait::TurnEnd { timeout_ms } => observed_outcome(
                prompt_turn(
                    backend,
                    execution.target.address.hub_id,
                    execution.target,
                    execution.baseline,
                    timeout_ms,
                    wake,
                )
                .await,
            ),
        },
    };
    Ok(AgentPromptResult {
        receipt: execution.receipt,
        terminal: execution.terminal,
        outcome,
        snapshot,
    })
}

type Observation = (AgentPromptOutcome, Option<AgentSnapshot>);

/// Once native submission has returned a receipt, a failed follow-up read
/// describes the turn observation. Keep the original delivery receipt.
pub(super) fn observed_outcome(result: Result<Observation, PublicError>) -> Observation {
    match result {
        Ok(observation) => observation,
        Err(PublicError::NotFound) => (AgentPromptOutcome::Exited, None),
        Err(_) => (AgentPromptOutcome::Unknown, None),
    }
}

/// Wait until the execution leaves `working`, asks for approval, or ends.
/// The cursor is the observation seen before submission, so only newer states
/// satisfy the condition. `timeout_ms` bounds the whole wait.
pub async fn prompt_turn<S: AgentSnapshots>(
    source: &S,
    local: HubId,
    target: AgentTarget,
    baseline: ObservationCursor,
    timeout_ms: u32,
    wake: watch::Receiver<u64>,
) -> Result<(AgentPromptOutcome, Option<AgentSnapshot>), PublicError> {
    let deadline = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
    let condition = AgentWaitCondition::Any {
        filters: vec![
            AgentWaitCondition::Activity {
                is: vec![
                    Activity::Idle,
                    Activity::WaitingForInput,
                    Activity::WaitingForApproval,
                ],
            },
            AgentWaitCondition::Lifecycle {
                is: vec![AgentLifecycleState::Exited],
            },
        ],
    };
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok((AgentPromptOutcome::TimedOut, None));
        }
        let wait_ms = remaining
            .as_millis()
            .clamp(1, u128::from(MAX_AGENT_WAIT_MS)) as u32;
        let request = AgentWaitRequest {
            targets: vec![target],
            runs: Vec::new(),
            condition: condition.clone(),
            after: Some(baseline),
            wait_ms,
        };
        let result = wait(
            source,
            local,
            &request,
            wake.clone(),
            Instant::now() + Duration::from_millis(u64::from(wait_ms)),
        )
        .await?;
        if let Some(snapshot) = result.matched.into_iter().next() {
            let outcome = match snapshot.fields().activity {
                Activity::WaitingForApproval => AgentPromptOutcome::Blocked,
                _ => AgentPromptOutcome::TurnEnded,
            };
            return Ok((outcome, Some(snapshot)));
        }
        if !result.ended.is_empty() || !result.timed_out {
            return Ok((AgentPromptOutcome::Exited, None));
        }
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/prompt_outcome.rs"]
mod outcome_tests;
