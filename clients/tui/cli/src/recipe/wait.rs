//! Recipe wait edges: a dependent start is held until every named predecessor
//! reaches a bounded observed state on its current execution. The wait never
//! infers readiness from elapsed time; it only accepts Hub observations.

use super::document::{RecipeAgent, RecipeWaitUntil};
use crate::constants::*;
use anyhow::{Result, anyhow, bail};
use std::time::Duration;
use tokio::time::Instant;
use yolo_hub_agent_protocol::{
    Activity, AgentLifecycleState, AgentStartResult, AgentWaitCondition, AgentWaitRequest,
    MAX_AGENT_WAIT_MS,
};
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunSelector, AgentRunState};
use super::run_query::{get_run, run_exited};
use yolo_hub_client::{ClientError, EndpointClient};
use yolo_hub_identity::{AgentTarget, HubId};
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_terminal_protocol::PublicError;

/// One run edge. The execution is captured once, so a replacement cannot
/// silently satisfy a completion edge.
struct Dependency {
    address: AgentRunAddress,
    target: Option<AgentTarget>,
}

/// Wait for every `after` run before the dependent agent starts.
pub(super) async fn dependencies(
    client: &mut EndpointClient,
    hub: HubId,
    agent: &RecipeAgent,
    started: &[AgentStartResult],
) -> Result<()> {
    let Some(wait) = agent.wait else {
        return Ok(());
    };
    let mut pending = Vec::with_capacity(agent.after.len());
    for name in &agent.after {
        let run = started
            .iter()
            .filter_map(|start| start.run.as_ref())
            .find(|run| run.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow!(RECIPE_DEPENDENCY_RUN_MISSING))?;
        pending.push(Dependency {
            address: run.address,
            target: None,
        });
    }
    let timeout = Duration::from_secs(u64::from(wait.timeout_seconds));
    wait_for(client, hub, pending, wait.until, timeout).await?;
    Ok(())
}

/// Wait for one run's current execution to reach `until` and return the
/// execution target so a prompt can capture its observation cursor.
pub(super) async fn run_state(
    client: &mut EndpointClient,
    hub: HubId,
    address: AgentRunAddress,
    until: RecipeWaitUntil,
    timeout: Duration,
) -> Result<AgentTarget> {
    let dependency = Dependency {
        address,
        target: None,
    };
    wait_for(client, hub, vec![dependency], until, timeout)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!(RECIPE_PROMPT_TARGET_MISSING))
}

async fn wait_for(
    client: &mut EndpointClient,
    hub: HubId,
    mut pending: Vec<Dependency>,
    until: RecipeWaitUntil,
    timeout: Duration,
) -> Result<Vec<AgentTarget>> {
    let deadline = Instant::now() + timeout;
    let condition = condition(until);
    let mut matched = Vec::new();
    while !pending.is_empty() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            bail!(RECIPE_WAIT_TIMEOUT);
        }
        let share = (remaining.as_millis() / pending.len() as u128)
            .clamp(1, u128::from(MAX_AGENT_WAIT_MS)) as u32;
        let mut finished = Vec::new();
        for dependency in &mut pending {
            let run = get_run(client, hub, dependency.address).await?;
            match run.state {
                AgentRunState::Stopped if until == RecipeWaitUntil::Exited => {
                    finished.push(dependency.address);
                    continue;
                }
                AgentRunState::Stopped => bail!(RECIPE_WAIT_RUN_STOPPED),
                _ => {}
            }
            let request = AgentWaitRequest {
                targets: dependency.target.into_iter().collect(),
                runs: if dependency.target.is_some() {
                    Vec::new()
                } else {
                    vec![AgentRunSelector::Address(dependency.address)]
                },
                condition: condition.clone(),
                after: None,
                wait_ms: share,
            };
            let call_deadline =
                Instant::now() + Duration::from_millis(u64::from(share)) + CATALOG_TIMEOUT;
            let reply = client
                .call(hub, Rpc::WaitAgents { request }, call_deadline)
                .await;
            let wait = match reply {
                Ok(Reply::AgentWait { wait }) => wait,
                Ok(_) => bail!(UNEXPECTED_RECIPE_WAIT_REPLY),
                Err(ClientError::Remote(PublicError::NotFound)) => {
                    if until == RecipeWaitUntil::Exited && run_exited(client, &run).await? {
                        finished.push(dependency.address);
                    } else {
                        // The pinned execution was replaced; re-resolve the
                        // run so the wait follows its current execution.
                        dependency.target = None;
                    }
                    continue;
                }
                Err(ClientError::Remote(PublicError::Conflict)) => continue,
                Err(error) => return Err(error.into()),
            };
            if dependency.target.is_none() {
                dependency.target = wait.targets.first().copied();
            }
            let Some(target) = dependency.target else {
                continue;
            };
            if wait
                .matched
                .iter()
                .any(|snapshot| snapshot.cursor().target() == target)
            {
                matched.push(target);
                finished.push(dependency.address);
            } else if until == RecipeWaitUntil::Exited && wait.ended.contains(&target) {
                let run = get_run(client, hub, dependency.address).await?;
                if run.state == AgentRunState::Stopped || run_exited(client, &run).await? {
                    finished.push(dependency.address);
                } else {
                    dependency.target = None;
                }
            } else if !wait.ended.is_empty() {
                bail!(RECIPE_WAIT_EXECUTION_ENDED);
            }
        }
        let progressed = !finished.is_empty();
        pending.retain(|dependency| !finished.contains(&dependency.address));
        if !progressed {
            tokio::time::sleep(remaining.min(RECIPE_WAIT_POLL)).await;
        }
    }
    Ok(matched)
}

fn condition(until: RecipeWaitUntil) -> AgentWaitCondition {
    match until {
        RecipeWaitUntil::Promptable => AgentWaitCondition::Promptable,
        RecipeWaitUntil::Idle => AgentWaitCondition::Activity {
            is: vec![Activity::Idle],
        },
        RecipeWaitUntil::Exited => AgentWaitCondition::Lifecycle {
            is: vec![AgentLifecycleState::Exited],
        },
    }
}
