//! Claude PermissionRequest bridge. A pending prompt is forwarded as a typed
//! interaction and blocks this hook until the Hub resolves it or the wait
//! expires; no decision means Claude's normal permission flow proceeds.

use super::hooks::{HookInput, canonical_session};
use super::native_client::NativeClient;
use crate::claude_hooks::{
    CLAUDE_NATIVE_SCHEMA_VERSION, CLAUDE_PERMISSION_NOT_READY_WAIT,
    CLAUDE_PERMISSION_RETRY_DELAY, CLAUDE_PERMISSION_WAIT, ClaudeLifecycleEvent,
    ClaudeNativeRequest, INITIAL_CLAUDE_EVENT_SEQUENCE, MAX_CLAUDE_PERMISSION_ENVELOPE_BYTES,
    MAX_CLAUDE_PERMISSION_TOOL_NAME_BYTES,
};
use anyhow::Result;
use uuid::Uuid;

pub(super) async fn run(input: &HookInput) -> Result<()> {
    if input.agent_id.is_some() {
        return Ok(());
    }
    let Some(tool_name) = input.tool_name.as_deref().filter(|name| {
        !name.is_empty()
            && name.len() <= MAX_CLAUDE_PERMISSION_TOOL_NAME_BYTES
            && !name.chars().any(char::is_control)
    }) else {
        return Ok(());
    };
    let Ok(provider_session_id) = canonical_session(&input.session_id) else {
        return Ok(());
    };
    let Some(client) = NativeClient::from_environment()? else {
        return Ok(());
    };
    let Some(allow) = request_decision(
        &client,
        provider_session_id,
        tool_name,
        input.tool_input.as_ref(),
    )
    .await?
    else {
        return Ok(());
    };
    let behavior = if allow { "allow" } else { "deny" };
    println!(
        "{}",
        serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PermissionRequest",
                "decision": { "behavior": behavior }
            }
        })
    );
    Ok(())
}

pub(crate) async fn request_decision(
    client: &NativeClient,
    provider_session_id: String,
    tool_name: &str,
    input: Option<&serde_json::Value>,
) -> Result<Option<bool>> {
    let interaction = Uuid::new_v4();
    let request = ClaudeNativeRequest {
        schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
        scope: client.scope,
        event_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE,
        event: ClaudeLifecycleEvent::InteractionRequested {
            event_id: Uuid::new_v4(),
            provider_session_id,
            interaction,
            tool_name: tool_name.to_owned(),
            prompt: prompt(tool_name, input),
        },
    };
    let deadline = tokio::time::Instant::now() + CLAUDE_PERMISSION_WAIT;
    let retry_deadline = tokio::time::Instant::now() + CLAUDE_PERMISSION_NOT_READY_WAIT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let Ok(Ok(response)) = tokio::time::timeout(remaining, client.send(&request)).await else {
            return Ok(None);
        };
        if response.accepted {
            return Ok(response
                .interaction
                .filter(|decision| decision.interaction_id == interaction)
                .map(|decision| decision.allow));
        }
        // The first tool turn can arrive before agent-binding.json is
        // published. Retry briefly so the interaction still reaches the Hub
        // once the SessionStart outbox replay binds the agent.
        if tokio::time::Instant::now() >= retry_deadline {
            return Ok(None);
        }
        tokio::time::sleep(CLAUDE_PERMISSION_RETRY_DELAY).await;
    }
}

fn prompt(tool_name: &str, input: Option<&serde_json::Value>) -> String {
    let mut text = tool_name.to_owned();
    if let Some(value) = input
        && let Ok(encoded) = serde_json::to_string(value)
    {
        text.push_str(": ");
        text.push_str(&encoded);
    }
    truncate(text, MAX_CLAUDE_PERMISSION_ENVELOPE_BYTES)
}

fn truncate(mut text: String, maximum: usize) -> String {
    if text.len() <= maximum {
        return text;
    }
    let mut end = maximum;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}
