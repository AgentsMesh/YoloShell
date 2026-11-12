use yolo_agent_catalog::AgentRow;
use crate::constants::*;
use crate::outcome::Outcome;
use crate::wait::{WaitOutcome, WaitResult};

pub use crate::conversation_text::{assistant_text, conversation_text};
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;
use yolo_conversation_protocol::ConversationEntry;
use yolo_hub_agent_protocol::{
    Activity, AgentSnapshot, Capability, CommandOutcome, CommandReceipt, Lifecycle,
};
use yolo_hub_identity::AgentTarget;

#[derive(Serialize)]
struct AgentRowJson<'a> {
    terminal_name: Option<&'a str>,
    agent: &'a AgentSnapshot,
}

#[derive(Serialize)]
struct AgentsView<'a> {
    agents: Vec<AgentRowJson<'a>>,
}

#[derive(Serialize)]
struct AgentView<'a> {
    terminal_name: Option<&'a str>,
    agent: &'a AgentSnapshot,
}

#[derive(Serialize)]
struct AskView {
    request_id: Uuid,
    target: AgentTarget,
    receipt: CommandOutcome,
    wait: &'static str,
    activity: String,
    assistant_text: Option<String>,
    entries: Vec<ConversationEntry>,
}

pub fn rows_json(rows: &[AgentRow]) -> Result<Value> {
    let agents = rows
        .iter()
        .map(|row| AgentRowJson {
            terminal_name: row.terminal_name.as_deref(),
            agent: &row.snapshot,
        })
        .collect();
    Ok(serde_json::to_value(AgentsView { agents })?)
}

pub fn agent_json(snapshot: &AgentSnapshot, terminal_name: Option<&str>) -> Result<Value> {
    Ok(serde_json::to_value(AgentView {
        terminal_name,
        agent: snapshot,
    })?)
}

pub fn rows_text(rows: &[AgentRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{}\t{}\t{}\t{}",
                row.terminal_name.as_deref().unwrap_or("-"),
                activity_label(row.snapshot.fields().activity),
                provider_label(&row.snapshot),
                row.snapshot.cursor().target()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// List output keeps lifecycle visible: `--all` includes history, and a
/// historical agent can still carry a current observation for its terminal.
pub fn agents_text(rows: &[AgentRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{}\t{}\t{}\t{}\t{}",
                row.terminal_name.as_deref().unwrap_or("-"),
                activity_label(row.snapshot.fields().activity),
                lifecycle_label(row.snapshot.fields().lifecycle),
                provider_label(&row.snapshot),
                row.snapshot.cursor().target()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn agent_text(snapshot: &AgentSnapshot, terminal_name: Option<&str>) -> String {
    let fields = snapshot.fields();
    format!(
        "terminal: {}\nagent: {}\nprovider: {}\nlifecycle: {}\nactivity: {}\nfreshness: {:?}\nreachability: {:?}\nsource: {:?}\nrecovery: {:?}\ncapabilities: interrupt={} submit_prompt={} respond_interaction={}",
        terminal_name.unwrap_or("-"),
        snapshot.cursor().target(),
        provider_label(snapshot),
        lifecycle_label(fields.lifecycle),
        activity_label(fields.activity),
        fields.freshness,
        fields.reachability,
        fields.source,
        fields.recovery,
        capability_label(fields.capabilities.interrupt),
        capability_label(fields.capabilities.submit_prompt),
        capability_label(fields.capabilities.respond_interaction)
    )
}

pub fn provider_label(snapshot: &AgentSnapshot) -> &str {
    let provider = &snapshot.fields().provider;
    provider.display_name().unwrap_or_else(|| provider.kind())
}

pub fn lifecycle_label(lifecycle: Lifecycle) -> &'static str {
    match lifecycle {
        Lifecycle::Unknown {} => "unknown",
        Lifecycle::Starting {} => "starting",
        Lifecycle::Running {} => "running",
        Lifecycle::Exited { .. } => "exited",
    }
}

pub fn activity_label(activity: Activity) -> String {
    serde_json::to_value(activity)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub fn capability_label(capability: Capability) -> String {
    match capability {
        Capability::Available {} => "available".into(),
        Capability::Unsupported {} => "unsupported".into(),
        Capability::Unavailable { reason } => format!("unavailable ({reason:?})"),
    }
}

pub fn ask_outcome(
    request_id: Uuid,
    target: AgentTarget,
    receipt: CommandReceipt,
    result: WaitResult,
    json_mode: bool,
) -> Result<Outcome> {
    let (code, label) = match result.outcome {
        WaitOutcome::Completed => (EXIT_SUCCESS, COMPLETED_LABEL),
        WaitOutcome::Approval => (EXIT_SUCCESS, APPROVAL_LABEL),
        WaitOutcome::Exited => (EXIT_SUCCESS, EXITED_LABEL),
        WaitOutcome::ExecutionEnded => (EXIT_REJECTED, EXECUTION_ENDED_LABEL),
        WaitOutcome::TimedOut => (EXIT_TIMEOUT, TIMED_OUT_LABEL),
    };
    let activity = result
        .snapshot
        .as_ref()
        .map(|snapshot| activity_label(snapshot.fields().activity))
        .unwrap_or_else(|| "unknown".to_owned());
    let body = assistant_text(&result.entries);
    let value = serde_json::to_value(AskView {
        request_id,
        target,
        receipt: receipt.outcome(),
        wait: label,
        activity: activity.clone(),
        assistant_text: body.clone(),
        entries: result.entries,
    })?;
    let text = format!(
        "{}\n{label}; activity={activity}; request {request_id}",
        body.as_deref().unwrap_or("(no assistant text observed)")
    );
    let mut outcome = Outcome::success(json_mode, value, text);
    outcome.code = code;
    Ok(outcome)
}
