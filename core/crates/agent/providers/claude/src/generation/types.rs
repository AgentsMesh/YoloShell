use super::termination::{SessionEndBoundary, SessionEndReason, SessionEndTermination};
use crate::agent::AgentState;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use uuid::Uuid;

pub use yolo_agent_authority_contract::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationMode {
    Fresh,
    Resume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStartSource {
    Fresh,
    Resume,
    Clear,
    Compact,
    Fork,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationBegin {
    #[serde(with = "crate::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub hook_token: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub helper_capability: Uuid,
    pub worker: AuthorityProcessIdentity,
    pub process: AuthorityProcessIdentity,
    pub settings_identity: crate::paths::identity::FileIdentity,
    pub handshake_boot_id: String,
    pub handshake_deadline_ns: u64,
    pub provider_session_id: String,
    pub discover_session: bool,
    pub mode: InvocationMode,
    pub cwd: PathBuf,
    pub data_home: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityAuthentication {
    #[serde(with = "crate::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub hook_token: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub helper_capability: Uuid,
    pub process: AuthorityProcessIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GenerationEvent {
    SessionStart {
        #[serde(with = "crate::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        source: SessionStartSource,
        cwd: PathBuf,
        transcript_path: PathBuf,
        state: AgentState,
    },
    StateChanged {
        #[serde(with = "crate::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        state: AgentState,
    },
    CwdChanged {
        #[serde(with = "crate::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        cwd: PathBuf,
        transcript_path: PathBuf,
    },
    SessionEnd {
        #[serde(with = "crate::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        reason: SessionEndReason,
        boundary: SessionEndBoundary,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedGenerationEvent {
    pub sequence: u64,
    pub generation: u64,
    pub event: GenerationEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationRecoveryEvent {
    pub sequence: u64,
    pub event: GenerationEvent,
    pub transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationApplyDisposition {
    Applied,
    Invalidated,
    Replay,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeGenerationAuthority {
    pub schema_version: u32,
    #[serde(with = "crate::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub hook_token: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub helper_capability: Uuid,
    pub worker: AuthorityProcessIdentity,
    pub process: AuthorityProcessIdentity,
    pub settings_identity: crate::paths::identity::FileIdentity,
    pub handshake_boot_id: String,
    pub handshake_deadline_ns: u64,
    pub revision: u64,
    pub generation: u64,
    pub last_event_sequence: u64,
    pub provider_session_id: String,
    pub discover_session: bool,
    pub mode: InvocationMode,
    pub cwd: PathBuf,
    pub data_home: PathBuf,
    pub transcript_path: Option<PathBuf>,
    pub transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    pub binding_revision: Option<u64>,
    pub state: AgentState,
    #[serde(deserialize_with = "required_termination_candidate")]
    pub termination_candidate: Option<SessionEndTermination>,
    pub replay_events: VecDeque<AppliedGenerationEvent>,
}

fn required_termination_candidate<'de, D>(
    deserializer: D,
) -> Result<Option<SessionEndTermination>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<SessionEndTermination>::deserialize(deserializer)
}
