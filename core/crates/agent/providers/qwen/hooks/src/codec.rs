//! Qwen Code's mapping onto the provider-neutral hook ingress. The engine owns
//! delivery, admission, durability and generation application; this module
//! only decodes Qwen payloads and declares Qwen's ingress policy.
use super::constants::*;
pub use super::constants::PROVIDER;
use super::contract::{
    QwenHookEvent, QwenHookPayload, QwenSessionEndReason, QwenSessionStartSource,
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use yolo_agent_adapter_contract::DetectionEnvironment;
use yolo_agent_contract::{AgentKind, ObservedProcess};
use yolo_hook_codec::{
    CanonicalHookEvent, HookAncestry, HookCodec, HookIngressProfile, OUTBOX_ENV, SOCKET_ENV,
};
use yolo_hooks_runtime::{EventKind, NotificationKind, ProviderVersion};

pub const PROFILE: HookIngressProfile = HookIngressProfile {
    provider: PROVIDER,
    socket_env: SOCKET_ENV,
    outbox_env: OUTBOX_ENV,
    max_ancestors: MAX_ANCESTORS,
    ancestry: HookAncestry::DetachedSession,
    require_peer_environment: true,
};

#[derive(Debug, Default)]
pub struct QwenHookCodec;

impl HookCodec for QwenHookCodec {
    fn profile(&self) -> HookIngressProfile {
        PROFILE
    }

    fn kind(&self) -> AgentKind {
        AgentKind::Other(yolo_qwen::qwen::constants::KIND.into())
    }

    fn provider_version(&self) -> ProviderVersion {
        ProviderVersion::new(PROVIDER_MAJOR, PROVIDER_MINOR, PROVIDER_PATCH)
    }

    fn executable(&self) -> &'static str {
        yolo_qwen::qwen::constants::EXECUTABLE
    }

    fn data_home(&self, agent: &ObservedProcess) -> Result<PathBuf> {
        let environment = agent
            .environment
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(INVALID))?;
        DetectionEnvironment::from_strings(environment)
            .data_home(
                yolo_qwen::qwen::constants::DATA_HOME_ENV,
                yolo_qwen::qwen::constants::DEFAULT_DATA_HOME_DIR,
            )
            .ok_or_else(|| anyhow::anyhow!(INVALID))
    }

    fn decode(&self, payload: &serde_json::Value) -> Result<Option<CanonicalHookEvent>> {
        let payload: QwenHookPayload = serde_json::from_value(payload.clone())?;
        payload.validate()?;
        let Some(event) = payload.event()? else {
            return Ok(None);
        };
        Ok(Some(CanonicalHookEvent {
            provider_session_id: payload.session_id.clone(),
            cwd: payload.cwd.clone(),
            transcript_path: payload.transcript_path.clone(),
            event: event_kind(event),
            turn_id: payload.prompt_id.clone(),
            agent_id: payload.agent_id().map(str::to_owned),
            tool_name: payload.tool_name.clone(),
            tool_use_id: payload.tool_use_id().map(str::to_owned),
            stop_hook_active: payload.stop_hook_active,
        }))
    }
}

pub fn codec() -> Arc<dyn HookCodec> {
    Arc::new(QwenHookCodec)
}

pub fn event_kind(event: QwenHookEvent) -> EventKind {
    match event {
        QwenHookEvent::SessionStart { source } => EventKind::SessionStart {
            source: match source {
                QwenSessionStartSource::Startup => yolo_hooks_runtime::SessionStartSource::Startup,
                QwenSessionStartSource::Resume => yolo_hooks_runtime::SessionStartSource::Resume,
                QwenSessionStartSource::Clear => yolo_hooks_runtime::SessionStartSource::Clear,
                QwenSessionStartSource::Compact => yolo_hooks_runtime::SessionStartSource::Compact,
                QwenSessionStartSource::Fork => yolo_hooks_runtime::SessionStartSource::Fork,
            },
        },
        QwenHookEvent::SessionEnd { reason } => EventKind::SessionEnd {
            reason: match reason {
                QwenSessionEndReason::Other => yolo_hooks_runtime::SessionEndReason::Other,
                QwenSessionEndReason::Clear => yolo_hooks_runtime::SessionEndReason::Clear,
                QwenSessionEndReason::Resume => yolo_hooks_runtime::SessionEndReason::Resume,
            },
        },
        QwenHookEvent::UserPromptSubmit => EventKind::UserPromptSubmit,
        QwenHookEvent::PreToolUse => EventKind::PreToolUse,
        QwenHookEvent::PostToolUse => EventKind::PostToolUse,
        QwenHookEvent::PostToolUseFailure => EventKind::PostToolUseFailure,
        QwenHookEvent::PermissionRequest => EventKind::PermissionRequest,
        QwenHookEvent::PermissionDenied => EventKind::PermissionDenied,
        QwenHookEvent::PreCompact => EventKind::PreCompact,
        QwenHookEvent::PostCompact => EventKind::PostCompact,
        QwenHookEvent::SubagentStart => EventKind::SubagentStart,
        QwenHookEvent::SubagentStop => EventKind::SubagentStop,
        QwenHookEvent::TurnEnded => EventKind::TurnEnded,
        QwenHookEvent::NotificationPermission => EventKind::Notification {
            kind: NotificationKind::PermissionPrompt,
        },
    }
}
