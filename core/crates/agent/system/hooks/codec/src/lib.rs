//! Provider-owned hook wire decoding contract. The ingress engine owns
//! delivery, admission, durability, publication and generation application;
//! a provider owns only the mapping from its hook payload to this canonical
//! vocabulary and its ingress policy profile.
use anyhow::Result;
use std::path::PathBuf;
use yolo_agent_contract::{AgentKind, ObservedProcess};
use yolo_hooks_runtime::{EventKind, ProviderVersion};

/// Socket environment variable shared by every generic ingress provider.
pub const SOCKET_ENV: &str = "YOLO_HOOK_INGRESS_SOCKET";
/// Outbox environment variable shared by every generic ingress provider.
pub const OUTBOX_ENV: &str = "YOLO_HOOK_INGRESS_OUTBOX";

/// Provider-owned mapping from its own hook payload to the canonical event
/// vocabulary. The engine owns delivery, admission, durability and publication.
pub trait HookCodec: Send + Sync {
    fn profile(&self) -> HookIngressProfile;
    fn provider(&self) -> &'static str {
        self.profile().provider
    }
    fn kind(&self) -> AgentKind;
    fn provider_version(&self) -> ProviderVersion;
    /// Binding `executable` token for this provider (the provider install token).
    fn executable(&self) -> &'static str;
    /// Provider data home derived from the live agent's environment.
    fn data_home(&self, agent: &ObservedProcess) -> Result<PathBuf>;
    /// `None` means the provider event carries no lifecycle fact and is ignored.
    fn decode(&self, payload: &serde_json::Value) -> Result<Option<CanonicalHookEvent>>;
    /// Some providers omit the transcript path on a fresh session's start
    /// event because the file does not exist yet. The codec may derive it
    /// from the live agent and the event once admission has identified it.
    fn resolve_transcript_path(
        &self,
        _agent: &ObservedProcess,
        event: &CanonicalHookEvent,
    ) -> Result<PathBuf> {
        Ok(event.transcript_path.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalHookEvent {
    pub provider_session_id: String,
    pub cwd: PathBuf,
    pub transcript_path: PathBuf,
    pub event: EventKind,
    pub turn_id: Option<String>,
    /// Provider subagent identity when the payload carries one.
    pub agent_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub stop_hook_active: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookAncestry {
    /// The callback runs in its own session/PTY (Qwen Code style).
    DetachedSession,
    /// The callback runs in the agent's foreground process group.
    ForegroundGroup,
}

/// Provider-neutral ingress policy. Each hook-install profile supplies one.
#[derive(Clone, Copy, Debug)]
pub struct HookIngressProfile {
    pub provider: &'static str,
    pub socket_env: &'static str,
    pub outbox_env: &'static str,
    pub max_ancestors: usize,
    pub ancestry: HookAncestry,
    /// Whether the callback process itself must carry the terminal fence;
    /// providers that sanitize their callback environment rely on ancestry.
    pub require_peer_environment: bool,
}

pub const CALLBACK_SHELLS: &[&str] = yolo_agent_runtime::NATIVE_CALLBACK_SHELLS;
