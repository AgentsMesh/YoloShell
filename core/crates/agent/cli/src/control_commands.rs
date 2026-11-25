use yolo_agent_baseline as baseline;
use yolo_agent_catalog as catalog;
use crate::constants::*;
use crate::control;
use crate::observe;
use crate::outcome::Outcome;
use crate::receipt::{self, Execution};
use crate::reject::rejected;
use crate::render;
use crate::wait;
use anyhow::{Result, bail};
use std::io::Read;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentPromptFields, AgentPromptOutcome, AgentPromptReference,
    AgentPromptRequest, AgentPromptResult, AgentPromptWait, CommandOutcome, Prompt,
};
use yolo_hub_client::LocalClient;
use yolo_hub_rpc_protocol::{Command, Reply};

pub async fn send(
    client: &mut LocalClient,
    selector: String,
    prompt: Option<String>,
    stdin: bool,
    take_control: bool,
    json: bool,
) -> Result<Outcome> {
    let text = prompt_text(prompt, stdin)?;
    let selection = catalog::resolve_live(client, &selector).await?;
    let snapshot = observe::snapshot(client, selection.target).await?;
    let request = control::prompt_request(&snapshot, text)?;
    let request_id = request.fields().request_id;
    let target = request.fields().target;
    let grant = control::acquire(client, snapshot.fields().terminal, take_control).await?;
    let execution = control::execute(client, request, &grant).await;
    control::release(client, &grant).await;
    receipt::outcome(execution?, request_id, target, json)
}

pub async fn ask(
    client: &mut LocalClient,
    selector: String,
    prompt: Option<String>,
    stdin: bool,
    take_control: bool,
    timeout: u64,
    json: bool,
) -> Result<Outcome> {
    let text = prompt_text(prompt, stdin)?;
    let selection = catalog::resolve_live(client, &selector).await?;
    let snapshot = observe::snapshot(client, selection.target).await?;
    let runtime = snapshot.fields().terminal;
    let baseline = baseline::capture(client, selection.target, runtime).await?;
    let request = control::prompt_request(&snapshot, text)?;
    let request_id = request.fields().request_id;
    let target = request.fields().target;
    let grant = control::acquire(client, runtime, take_control).await?;
    let execution = control::execute(client, request, &grant).await;
    control::release(client, &grant).await;
    let execution = execution?;
    let Execution::Receipt(receipt) = execution else {
        return receipt::unknown_outcome(request_id, target, json);
    };
    if !matches!(
        receipt.outcome(),
        CommandOutcome::Delivered {} | CommandOutcome::Confirmed {} | CommandOutcome::Accepted {}
    ) {
        return receipt::outcome(Execution::Receipt(receipt), request_id, target, json);
    }
    let result = wait::complete(
        client,
        target,
        runtime,
        baseline,
        Duration::from_secs(timeout),
    )
    .await?;
    render::ask_outcome(request_id, target, receipt, result, json)
}

pub async fn prompt(
    client: &mut LocalClient,
    selector: String,
    prompt: Option<String>,
    stdin: bool,
    take_control: bool,
    wait_turn: bool,
    wait_ms: u32,
    json: bool,
) -> Result<Outcome> {
    let text = prompt_text(prompt, stdin)?;
    let selection = catalog::resolve_live(client, &selector).await?;
    let target = selection.target;
    let request = AgentPromptRequest::new(AgentPromptFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        reference: AgentPromptReference::Execution { target },
        prompt: Prompt::new(text).map_err(|_| rejected(PROMPT_INVALID))?,
        wait: if wait_turn {
            AgentPromptWait::TurnEnd { timeout_ms: wait_ms }
        } else {
            AgentPromptWait::None
        },
        take_control,
    })?;
    let deadline =
        Instant::now() + CONTROL_TIMEOUT + Duration::from_millis(u64::from(wait_ms));
    match client
        .call(
            target.address.hub_id,
            Command::AgentPrompt { request },
            deadline,
        )
        .await?
    {
        Reply::AgentPrompt { prompt } => {
            let value = serde_json::to_value(&prompt)?;
            Ok(Outcome::success(json, value, prompt_label(&prompt)))
        }
        _ => Err(rejected(UNEXPECTED_REPLY)),
    }
}

fn prompt_label(result: &AgentPromptResult) -> String {
    let label = match result.outcome {
        AgentPromptOutcome::Delivered => "delivered",
        AgentPromptOutcome::TurnEnded => "turn_ended",
        AgentPromptOutcome::Blocked => "blocked",
        AgentPromptOutcome::TimedOut => "timed_out",
        AgentPromptOutcome::Exited => "exited",
        AgentPromptOutcome::Rejected => "rejected",
        AgentPromptOutcome::Unknown => "unknown",
    };
    format!("{label} {}", result.receipt.target())
}

pub async fn interrupt(
    client: &mut LocalClient,
    selector: String,
    take_control: bool,
    json: bool,
) -> Result<Outcome> {
    let selection = catalog::resolve_live(client, &selector).await?;
    let snapshot = observe::snapshot(client, selection.target).await?;
    let request = control::interrupt_request(&snapshot)?;
    let request_id = request.fields().request_id;
    let target = request.fields().target;
    let grant = control::acquire(client, snapshot.fields().terminal, take_control).await?;
    let execution = control::execute(client, request, &grant).await;
    control::release(client, &grant).await;
    receipt::outcome(execution?, request_id, target, json)
}

pub async fn respond(
    client: &mut LocalClient,
    selector: String,
    choice: String,
    take_control: bool,
    json: bool,
) -> Result<Outcome> {
    let selection = catalog::resolve_live(client, &selector).await?;
    let snapshot = observe::snapshot(client, selection.target).await?;
    let request = control::respond_request(&snapshot, choice)?;
    let request_id = request.fields().request_id;
    let target = request.fields().target;
    let grant = control::acquire(client, snapshot.fields().terminal, take_control).await?;
    let execution = control::execute(client, request, &grant).await;
    control::release(client, &grant).await;
    receipt::outcome(execution?, request_id, target, json)
}

fn prompt_text(prompt: Option<String>, stdin: bool) -> Result<String> {
    if let Some(text) = prompt {
        return Ok(text);
    }
    if !stdin {
        bail!("a prompt argument or --stdin is required");
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(MAX_PROMPT_STDIN_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PROMPT_STDIN_BYTES {
        return Err(rejected(PROMPT_INPUT_LIMIT));
    }
    let mut text = String::from_utf8(bytes).map_err(|_| rejected(PROMPT_INPUT_TYPE))?;
    while text.ends_with('\n') || text.ends_with('\r') {
        text.pop();
    }
    Ok(text)
}
