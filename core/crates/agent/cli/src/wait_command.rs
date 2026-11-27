use yolo_agent_catalog::{self as catalog, AgentRow};
use crate::constants::*;
use crate::outcome::Outcome;
use crate::render;
use crate::reject::rejected;
use anyhow::Result;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::Instant;
use yolo_hub_agent_protocol::{
    Activity, AgentLifecycleState, AgentSnapshot, AgentWaitCondition, AgentWaitRequest, Freshness,
    MAX_AGENT_WAIT_MS, Reachability,
};
use yolo_hub_agent_run_protocol::AgentRunSelector;
use yolo_hub_client::LocalClient;
use yolo_hub_identity::AgentTarget;
use yolo_hub_rpc_protocol::{Command, Reply};

/// Wait until any `--until` state holds for any selected agent. The Hub
/// evaluates one bounded long-poll at a time; the client re-issues it until
/// the overall timeout, so a dropped connection never loses the deadline.
pub async fn wait(
    client: &mut LocalClient,
    selectors: Vec<String>,
    runs: Vec<String>,
    until: Vec<String>,
    timeout: u64,
    json: bool,
) -> Result<Outcome> {
    let selections = if selectors.is_empty() {
        Vec::new()
    } else {
        catalog::resolve_live_many(client, &selectors).await?
    };
    let hub = selections
        .first()
        .map(|selection| selection.target.address.hub_id)
        .unwrap_or_else(|| client.hello().hub_id);
    if selections
        .iter()
        .any(|selection| selection.target.address.hub_id != hub)
    {
        return Err(rejected(WAIT_MULTIPLE_HUBS));
    }
    let names: BTreeMap<AgentTarget, String> = selections
        .iter()
        .filter_map(|selection| {
            selection
                .terminal_name
                .clone()
                .map(|name| (selection.target, name))
        })
        .collect();
    let targets: Vec<AgentTarget> = selections.iter().map(|selection| selection.target).collect();
    let runs: Vec<AgentRunSelector> = runs
        .iter()
        .map(|run| crate::runs::selector(hub, run))
        .collect::<Result<_>>()?;
    let condition = condition(&until)?;
    let deadline = Instant::now() + Duration::from_secs(timeout);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(Outcome::failure(
                EXIT_TIMEOUT,
                json,
                WAIT_TIMEOUT.into(),
            ));
        }
        let wait_ms = remaining
            .as_millis()
            .clamp(1, u128::from(MAX_AGENT_WAIT_MS)) as u32;
        let request = AgentWaitRequest {
            targets: targets.clone(),
            runs: runs.clone(),
            condition: condition.clone(),
            after: None,
            wait_ms,
        };
        let call_deadline =
            Instant::now() + Duration::from_millis(u64::from(wait_ms)) + CONTROL_TIMEOUT;
        // Waits are evaluated by the Hub that owns the targets, exactly like
        // GetAgent: the entry Hub forwards a peer-addressed read.
        let reply = client
            .call(hub, Command::WaitAgents { request }, call_deadline)
            .await?;
        match reply {
            Reply::AgentWait { wait } if !wait.matched.is_empty() => {
                // Run selectors are expanded by the Hub, so the reply set is
                // the evaluated size, not the caller's positional targets.
                return Ok(matched(json, &wait.matched, &names, wait.targets.len())?);
            }
            Reply::AgentWait { wait } if !wait.ended.is_empty() => {
                return Err(rejected(EXECUTION_CHANGED))
            }
            Reply::AgentWait { wait } if wait.timed_out => {}
            _ => return Err(rejected(UNEXPECTED_REPLY)),
        }
    }
}

fn matched(
    json: bool,
    matched: &[AgentSnapshot],
    names: &BTreeMap<AgentTarget, String>,
    targets: usize,
) -> Result<Outcome> {
    if targets == 1 && matched.len() == 1 {
        let snapshot = &matched[0];
        let name = names
            .get(&snapshot.cursor().target())
            .map(String::as_str);
        return Ok(Outcome::success(
            json,
            render::agent_json(snapshot, name)?,
            render::agent_text(snapshot, name),
        ));
    }
    let rows: Vec<AgentRow> = matched
        .iter()
        .map(|snapshot| AgentRow {
            terminal_name: names.get(&snapshot.cursor().target()).cloned(),
            snapshot: snapshot.clone(),
        })
        .collect();
    Ok(Outcome::success(
        json,
        render::rows_json(&rows)?,
        render::rows_text(&rows),
    ))
}

fn condition(until: &[String]) -> Result<AgentWaitCondition> {
    let mut filters = Vec::with_capacity(until.len());
    for value in until {
        filters.push(match value.as_str() {
            STATE_IDLE => activity(Activity::Idle),
            STATE_WORKING => activity(Activity::Working),
            STATE_WAITING_FOR_INPUT | "prompt" => activity(Activity::WaitingForInput),
            STATE_WAITING_FOR_APPROVAL | "approval" => activity(Activity::WaitingForApproval),
            STATE_EXITED => AgentWaitCondition::Lifecycle {
                is: vec![AgentLifecycleState::Exited],
            },
            STATE_UNREACHABLE => AgentWaitCondition::Reachability {
                is: vec![Reachability::Unreachable],
            },
            STATE_STALE => AgentWaitCondition::Freshness {
                is: vec![Freshness::Stale],
            },
            STATE_PROMPTABLE => AgentWaitCondition::Promptable,
            STATE_INTERRUPTIBLE => AgentWaitCondition::Interruptible,
            STATE_RESPONDABLE => AgentWaitCondition::Respondable,
            other => return Err(rejected(format!("{INVALID_WAIT_STATE}: {other}"))),
        });
    }
    Ok(if filters.len() == 1 {
        filters.pop().expect("one filter")
    } else {
        AgentWaitCondition::Any { filters }
    })
}

fn activity(activity: Activity) -> AgentWaitCondition {
    AgentWaitCondition::Activity { is: vec![activity] }
}

