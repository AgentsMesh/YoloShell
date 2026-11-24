use super::constants::*;
use crate::claude_hooks::{
    CLAUDE_PERMISSION_REQUEST_EVENT, ClaudeHookState, ClaudeLifecycleEvent, ClaudeSessionEndReason,
    ClaudeSessionStartSource,
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct HookInput {
    pub(super) session_id: String,
    transcript_path: PathBuf,
    cwd: PathBuf,
    pub(super) hook_event_name: String,
    source: Option<String>,
    new_cwd: Option<PathBuf>,
    pub(super) agent_id: Option<String>,
    #[allow(dead_code)]
    agent_type: Option<String>,
    notification_type: Option<String>,
    reason: Option<String>,
    pub(super) tool_name: Option<String>,
    pub(super) tool_input: Option<serde_json::Value>,
}

pub async fn run() -> Result<()> {
    let input = read_input().await?;
    if input.hook_event_name == CLAUDE_PERMISSION_REQUEST_EVENT {
        return super::permission::run(&input).await;
    }
    let Some(event) = lifecycle(input)? else {
        return Ok(());
    };
    super::native_delivery::deliver(event).await
}

fn main_thread(input: &HookInput) -> bool {
    input.agent_id.is_none()
}

async fn read_input() -> Result<HookInput> {
    let mut input = tokio::io::stdin().take(HOOK_INPUT_MAX_BYTES.saturating_add(1));
    let mut bytes = Vec::new();
    input
        .read_to_end(&mut bytes)
        .await
        .context("read Claude hook input")?;
    ensure!(
        bytes.len() as u64 <= HOOK_INPUT_MAX_BYTES,
        "Claude hook input is too large"
    );
    serde_json::from_slice(&bytes).context("decode Claude hook input")
}

fn lifecycle(input: HookInput) -> Result<Option<ClaudeLifecycleEvent>> {
    if !main_thread(&input) {
        return Ok(None);
    }
    let event_id = Uuid::new_v4();
    Ok(Some(match input.hook_event_name.as_str() {
        SESSION_START_EVENT => ClaudeLifecycleEvent::SessionStart {
            event_id,
            provider_session_id: canonical_session(&input.session_id)?,
            source: start_source(input.source.as_deref())?,
            cwd: input.cwd,
            transcript_path: input.transcript_path,
            state: ClaudeHookState::WaitingForInput,
        },
        SESSION_END_EVENT => ClaudeLifecycleEvent::SessionEnd {
            event_id,
            provider_session_id: canonical_session(&input.session_id)?,
            reason: ClaudeSessionEndReason::from_provider(input.reason.as_deref()),
        },
        USER_PROMPT_EVENT => ClaudeLifecycleEvent::StateChanged {
            event_id,
            provider_session_id: canonical_session(&input.session_id)?,
            state: ClaudeHookState::Running,
        },
        // Claude Stop only means that a response stop was attempted. Without a
        // provider finalization proof it must not publish an idle state.
        STOP_EVENT => {
            canonical_session(&input.session_id)?;
            return Ok(None);
        }
        CWD_CHANGED_EVENT => ClaudeLifecycleEvent::CwdChanged {
            event_id,
            provider_session_id: canonical_session(&input.session_id)?,
            cwd: input.new_cwd.unwrap_or(input.cwd),
            transcript_path: input.transcript_path,
        },
        NOTIFICATION_EVENT => {
            let Some(kind) = input.notification_type.as_deref() else {
                return Ok(None);
            };
            if kind == PERMISSION_NOTIFICATION_TYPE || !WAITING_NOTIFICATION_TYPES.contains(&kind) {
                return Ok(None);
            }
            ClaudeLifecycleEvent::StateChanged {
                event_id,
                provider_session_id: canonical_session(&input.session_id)?,
                state: ClaudeHookState::WaitingForInput,
            }
        }
        PRE_TOOL_USE_EVENT => ClaudeLifecycleEvent::StateChanged {
            event_id,
            provider_session_id: canonical_session(&input.session_id)?,
            state: ClaudeHookState::Running,
        },
        _ => anyhow::bail!("unsupported Claude hook event"),
    }))
}

pub(super) fn canonical_session(value: &str) -> Result<String> {
    let parsed = Uuid::parse_str(value).context("Claude hook session is invalid")?;
    ensure!(
        parsed.hyphenated().to_string() == value,
        "Claude hook session is not canonical"
    );
    Ok(value.to_owned())
}

fn start_source(value: Option<&str>) -> Result<ClaudeSessionStartSource> {
    Ok(match value {
        Some(SOURCE_STARTUP) => ClaudeSessionStartSource::Fresh,
        Some(SOURCE_RESUME) => ClaudeSessionStartSource::Resume,
        Some(SOURCE_CLEAR) => ClaudeSessionStartSource::Clear,
        Some(SOURCE_COMPACT) => ClaudeSessionStartSource::Compact,
        Some(SOURCE_FORK) => ClaudeSessionStartSource::Fork,
        _ => anyhow::bail!("unsupported Claude session start source"),
    })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/hooks_tests.rs"]
mod tests;
