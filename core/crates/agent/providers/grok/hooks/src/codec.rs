use crate::constants::*;
use crate::events::event;
use crate::payload::{normalized_session_id, HookPayload};
use crate::transcript::session_transcript;
use anyhow::{Result, ensure};
use std::path::PathBuf;
use yolo_agent_contract::{AgentKind, ObservedProcess};
use yolo_hook_codec::{CanonicalHookEvent, HookCodec, HookIngressProfile};
use yolo_hooks_runtime::ProviderVersion;

#[derive(Debug, Default)]
pub struct GrokHookCodec;

impl HookCodec for GrokHookCodec {
    fn profile(&self) -> HookIngressProfile {
        PROFILE
    }

    fn kind(&self) -> AgentKind {
        AgentKind::Other(yolo_grok::grok::constants::KIND.into())
    }

    fn provider_version(&self) -> ProviderVersion {
        ProviderVersion::new(PROVIDER_MAJOR, PROVIDER_MINOR, PROVIDER_PATCH)
    }

    fn executable(&self) -> &'static str {
        yolo_grok::grok::constants::EXECUTABLE
    }

    fn data_home(&self, agent: &ObservedProcess) -> Result<PathBuf> {
        let environment = agent
            .environment
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(INVALID_PAYLOAD))?;
        yolo_grok::grok::data_home(environment).ok_or_else(|| anyhow::anyhow!(INVALID_PAYLOAD))
    }

    fn resolve_transcript_path(
        &self,
        agent: &ObservedProcess,
        event: &CanonicalHookEvent,
    ) -> Result<PathBuf> {
        if event.transcript_path.is_absolute() {
            return Ok(event.transcript_path.clone());
        }
        Ok(session_transcript(
            &self.data_home(agent)?,
            &event.cwd,
            &event.provider_session_id,
        ))
    }

    fn decode(&self, payload: &serde_json::Value) -> Result<Option<CanonicalHookEvent>> {
        let payload = HookPayload::new(payload);
        let session_id = payload
            .text("sessionId", PAYLOAD_SESSION_ID)
            .ok_or_else(|| anyhow::anyhow!(INVALID_PAYLOAD))?;
        let cwd = payload
            .path("cwd", "cwd")
            .ok_or_else(|| anyhow::anyhow!(INVALID_PAYLOAD))?;
        ensure!(
            normalized_session_id(session_id).is_some() && cwd.is_absolute(),
            INVALID_SESSION
        );
        let Some(event) = event(&payload)? else {
            return Ok(None);
        };
        Ok(Some(CanonicalHookEvent {
            provider_session_id: session_id.to_owned(),
            cwd,
            transcript_path: payload
                .path("transcriptPath", PAYLOAD_TRANSCRIPT_PATH)
                .unwrap_or_default(),
            event,
            agent_id: None,
            turn_id: payload.text("promptId", "prompt_id").map(str::to_owned),
            tool_name: payload.text("toolName", PAYLOAD_TOOL_NAME).map(str::to_owned),
            tool_use_id: payload.text("toolUseId", PAYLOAD_TOOL_USE_ID).map(str::to_owned),
            stop_hook_active: payload.flag("stopHookActive", PAYLOAD_STOP_HOOK_ACTIVE),
        }))
    }
}
