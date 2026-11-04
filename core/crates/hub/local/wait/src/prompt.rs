//! Turn wait for the atomic agent prompt. The caller submits through the
//! native control path; this helper evaluates a post-submission condition
//! against observations and never inspects screen text.

use std::time::Duration;
use tokio::sync::watch;
use tokio::time::Instant;
use yolo_hub_agent_protocol::{
    Activity, AgentLifecycleState, AgentPromptOutcome, AgentSnapshot, AgentWaitCondition,
    AgentWaitRequest, MAX_AGENT_WAIT_MS, ObservationCursor,
};
use yolo_hub_identity::{AgentTarget, HubId};
use yolo_hub_terminal_protocol::PublicError;

use crate::{AgentSnapshots, wait};

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
