//! Recipe prompts: after a start, wait for the run's observed promptable
//! capability, then submit one bounded prompt through the atomic method.
//! An optional turn wait uses the pre-submit observation cursor, so only a
//! newer observation can prove the turn ended. Unknown receipts are never
//! replayed.

use super::document::RecipeAgent;
use crate::constants::*;
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, Activity, AgentLifecycleState, AgentPromptFields,
    AgentPromptOutcome, AgentPromptReference, AgentPromptRequest, AgentPromptWait,
    AgentStartResult, AgentWaitCondition, AgentWaitRequest, MAX_AGENT_WAIT_MS, ObservationCursor,
    Prompt,
};
use yolo_hub_client::{ClientError, EndpointClient};
use yolo_hub_identity::{AgentTarget, HubId};
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecipePromptWait {
    #[default]
    Delivered,
    TurnEnd,
}

impl RecipePromptWait {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Delivered => RECIPE_PROMPT_DELIVERED,
            Self::TurnEnd => RECIPE_PROMPT_TURN_END,
        }
    }
}

pub(super) async fn apply(
    client: &mut EndpointClient,
    hub: HubId,
    agent: &RecipeAgent,
    start: &AgentStartResult,
) -> Result<()> {
    let Some(prompt) = &agent.prompt else {
        return Ok(());
    };
    let run = start
        .run
        .as_ref()
        .ok_or_else(|| anyhow!(RECIPE_PROMPT_REQUIRES_RUN))?;
    let timeout = Duration::from_secs(u64::from(prompt.timeout_seconds));
    let target = super::wait::run_state(
        client,
        hub,
        run.address,
        super::RecipeWaitUntil::Promptable,
        timeout,
    )
    .await?;
    let baseline = match prompt.wait {
        RecipePromptWait::Delivered => None,
        RecipePromptWait::TurnEnd => Some(baseline(client, hub, target).await?),
    };
    let request = AgentPromptRequest::new(AgentPromptFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        reference: AgentPromptReference::Execution { target },
        prompt: Prompt::new(prompt.text.clone()).map_err(|_| anyhow!(INVALID_RECIPE_PROMPT))?,
        wait: AgentPromptWait::None,
        take_control: false,
    })?;
    let command = Rpc::AgentPrompt { request };
    let deadline = Instant::now() + command.operation_timeout(CATALOG_TIMEOUT);
    match client.call(hub, command, deadline).await? {
        Reply::AgentPrompt { prompt: result }
            if matches!(
                result.outcome,
                AgentPromptOutcome::Delivered | AgentPromptOutcome::Blocked
            ) =>
        {
            match baseline {
                None => Ok(()),
                Some((target, cursor)) => turn_end(client, hub, target, cursor, timeout).await,
            }
        }
        Reply::AgentPrompt { .. } => bail!(RECIPE_PROMPT_REJECTED),
        _ => bail!(UNEXPECTED_RECIPE_REPLY),
    }
}

/// Capture the current execution and its observation cursor before submission.
async fn baseline(
    client: &mut EndpointClient,
    hub: HubId,
    target: AgentTarget,
) -> Result<(AgentTarget, ObservationCursor)> {
    let deadline = Instant::now() + CATALOG_TIMEOUT;
    match client.call(hub, Rpc::GetAgent { target }, deadline).await? {
        Reply::Agent { agent } => Ok((target, agent.cursor())),
        _ => bail!(UNEXPECTED_RECIPE_REPLY),
    }
}

/// A turn ends only on an observation newer than the pre-submit cursor.
async fn turn_end(
    client: &mut EndpointClient,
    hub: HubId,
    target: AgentTarget,
    baseline: ObservationCursor,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
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
            bail!(RECIPE_PROMPT_TURN_TIMEOUT);
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
        let call_deadline =
            Instant::now() + Duration::from_millis(u64::from(wait_ms)) + CATALOG_TIMEOUT;
        match client
            .call(hub, Rpc::WaitAgents { request }, call_deadline)
            .await
        {
            Ok(Reply::AgentWait { wait }) if !wait.matched.is_empty() || !wait.ended.is_empty() => {
                return Ok(());
            }
            Ok(Reply::AgentWait { wait }) if wait.timed_out => {}
            Err(ClientError::Remote(PublicError::NotFound)) => {
                bail!(RECIPE_WAIT_EXECUTION_ENDED)
            }
            Ok(_) => bail!(UNEXPECTED_RECIPE_REPLY),
            Err(error) => return Err(error.into()),
        }
    }
}
