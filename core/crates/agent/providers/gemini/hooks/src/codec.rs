use crate::constants::*;
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_agent_contract::{AgentKind, ObservedProcess};
use yolo_hook_codec::{CanonicalHookEvent, HookCodec, HookIngressProfile};
use yolo_hooks_runtime::{
    EventKind, NotificationKind, ProviderVersion, SessionEndReason, SessionStartSource,
};

#[derive(Debug, Deserialize)]
struct HookPayload {
    hook_event_name: String,
    session_id: String,
    transcript_path: PathBuf,
    cwd: PathBuf,
    source: Option<String>,
    reason: Option<String>,
    notification_type: Option<String>,
    tool_name: Option<String>,
    tool_use_id: Option<String>,
    stop_hook_active: Option<bool>,
}

#[derive(Debug, Default)]
pub struct GeminiHookCodec;

impl HookCodec for GeminiHookCodec {
    fn profile(&self) -> HookIngressProfile {
        PROFILE
    }

    fn kind(&self) -> AgentKind {
        AgentKind::Other(yolo_gemini::gemini::constants::KIND.into())
    }

    fn provider_version(&self) -> ProviderVersion {
        ProviderVersion::new(PROVIDER_MAJOR, PROVIDER_MINOR, PROVIDER_PATCH)
    }

    fn executable(&self) -> &'static str {
        yolo_gemini::gemini::constants::EXECUTABLE
    }

    fn data_home(&self, agent: &ObservedProcess) -> Result<PathBuf> {
        let environment = agent.environment.as_ref().ok_or_else(|| anyhow::anyhow!(INVALID_PAYLOAD))?;
        yolo_gemini::gemini::data_home(environment).ok_or_else(|| anyhow::anyhow!(INVALID_PAYLOAD))
    }

    fn decode(&self, payload: &serde_json::Value) -> Result<Option<CanonicalHookEvent>> {
        let payload = HookPayload::deserialize(payload)?;
        ensure!(
            canonical(&payload.session_id).is_some()
                && payload.cwd.is_absolute()
                && payload.transcript_path.is_absolute(),
            INVALID_SESSION
        );
        let Some(event) = event(&payload)? else {
            return Ok(None);
        };
        let transcript_path = resume_transcript(&payload, &event);
        Ok(Some(CanonicalHookEvent {
            provider_session_id: payload.session_id,
            cwd: payload.cwd,
            transcript_path,
            event,
            turn_id: None,
            agent_id: None,
            tool_name: payload.tool_name,
            tool_use_id: payload.tool_use_id,
            stop_hook_active: payload.stop_hook_active,
        }))
    }
}

fn resume_transcript(payload: &HookPayload, event: &EventKind) -> PathBuf {
    if matches!(
        event,
        EventKind::SessionStart {
            source: SessionStartSource::Resume
        }
    ) {
        if let Some(path) = yolo_gemini::gemini::resolve_transcript(
            &payload.transcript_path,
            &payload.session_id,
        ) {
            return path;
        }
    }
    payload.transcript_path.clone()
}

fn event(payload: &HookPayload) -> Result<Option<EventKind>> {
    let event = match payload.hook_event_name.as_str() {
        EVENT_SESSION_START => EventKind::SessionStart {
            source: session_source(payload.source.as_deref())?,
        },
        EVENT_SESSION_END => EventKind::SessionEnd {
            reason: session_end(payload.reason.as_deref()),
        },
        EVENT_BEFORE_AGENT => EventKind::UserPromptSubmit,
        EVENT_AFTER_AGENT => EventKind::TurnEnded,
        EVENT_BEFORE_TOOL => EventKind::PreToolUse,
        EVENT_AFTER_TOOL => EventKind::PostToolUse,
        EVENT_NOTIFICATION => EventKind::Notification {
            kind: notification(payload.notification_type.as_deref()),
        },
        // BeforeModel/AfterModel/BeforeToolSelection/PreCompress and any future
        // telemetry event carry no lifecycle fact.
        _ => return Ok(None),
    };
    Ok(Some(event))
}

fn session_source(source: Option<&str>) -> Result<SessionStartSource> {
    Ok(match source {
        Some("startup") | None => SessionStartSource::Startup,
        Some("resume") => SessionStartSource::Resume,
        Some("clear") => SessionStartSource::Clear,
        Some("compact") => SessionStartSource::Compact,
        Some(_) => anyhow::bail!(INVALID_PAYLOAD),
    })
}

fn session_end(reason: Option<&str>) -> SessionEndReason {
    match reason {
        Some("clear") => SessionEndReason::Clear,
        Some("resume") => SessionEndReason::Resume,
        _ => SessionEndReason::Other,
    }
}

fn notification(kind: Option<&str>) -> NotificationKind {
    match kind {
        // Gemini 0.66 emits `ToolPermission`; the other spellings are accepted
        // defensively for neighboring releases.
        Some("ToolPermission") | Some("tool_permission") | Some(NOTIFICATION_PERMISSION_PROMPT)
        | Some("permission") => NotificationKind::PermissionPrompt,
        Some(NOTIFICATION_IDLE_PROMPT) | Some(NOTIFICATION_AGENT_NEEDS_INPUT) => {
            NotificationKind::AgentNeedsInput
        }
        Some("elicitation") => NotificationKind::Elicitation,
        Some(other) => NotificationKind::Other(other.to_owned()),
        None => NotificationKind::Other("notification".to_owned()),
    }
}

fn canonical(value: &str) -> Option<String> {
    let parsed = Uuid::parse_str(value).ok()?;
    let canonical = parsed.hyphenated().to_string();
    (canonical == value).then_some(canonical)
}
