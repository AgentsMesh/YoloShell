use crate::HookViolation;
use crate::snapshot::event::wire::EventEnvelopeDto;
use crate::state::{
    AttentionState, InstrumentationState, ProcessState, TerminationCandidate, TurnState,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

fn deserialize_required_option<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotDocument {
    pub(crate) schema_version: u16,
    pub(crate) authority_revision: u64,
    pub(crate) binding: BindingDto,
    pub(crate) lifecycle: LifecycleDto,
    pub(crate) history: HistoryDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RevocationDto {
    HookEvent {
        predecessor: Box<SnapshotDocument>,
        #[serde(with = "crate::sha256_digest::compact")]
        predecessor_sha256: crate::sha256_digest::Sha256Digest,
        event: EventEnvelopeDto,
        event_id: Uuid,
        sequence: u64,
        violation: HookViolation,
        #[serde(with = "crate::sha256_digest::compact")]
        envelope_sha256: crate::sha256_digest::Sha256Digest,
    },
    ProcessObservation {
        predecessor: Box<SnapshotDocument>,
        #[serde(with = "crate::sha256_digest::compact")]
        predecessor_sha256: crate::sha256_digest::Sha256Digest,
        pid: u32,
        start_time: u64,
        violation: HookViolation,
        #[serde(with = "crate::sha256_digest::compact")]
        observation_sha256: crate::sha256_digest::Sha256Digest,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BindingDto {
    pub(crate) identity: IdentityDto,
    pub(crate) provider: ProviderDto,
    pub(crate) provider_version: VersionDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityDto {
    pub(crate) terminal_id: Uuid,
    pub(crate) runtime_epoch: Uuid,
    pub(crate) invocation_id: Uuid,
    pub(crate) hook_token: Uuid,
    pub(crate) process: ProcessIdentityDto,
    pub(crate) generation: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProcessIdentityDto {
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ProviderDto {
    Codex {},
    Claude {},
    Other { name: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionDto {
    pub(crate) major: u16,
    pub(crate) minor: u16,
    pub(crate) patch: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LifecycleDto {
    pub(crate) process: ProcessState,
    pub(crate) turn: TurnState,
    pub(crate) attention: AttentionState,
    pub(crate) instrumentation: InstrumentationState,
    pub(crate) provider_session_id: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub(crate) current_cwd: Option<PathBuf>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub(crate) current_transcript_path: Option<PathBuf>,
    pub(crate) last_sequence: u64,
    pub(crate) stop_candidate: Option<StopDto>,
    pub(crate) termination_candidate: Option<TerminationCandidate>,
    pub(crate) final_hook_seen: bool,
    pub(crate) active_tools: Vec<String>,
    pub(crate) children: Vec<ChildEntryDto>,
    pub(crate) compaction: Option<CompactionDto>,
    pub(crate) child_compactions: Vec<ChildCompactionDto>,
    pub(crate) revoked: bool,
    pub(crate) revocation: Option<RevocationDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StopDto {
    pub(crate) event_id: Uuid,
    pub(crate) turn_id: Option<String>,
    pub(crate) continued: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChildEntryDto {
    pub(crate) agent_id: String,
    pub(crate) lifecycle: ChildDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChildDto {
    pub(crate) active: bool,
    pub(crate) turn: TurnState,
    pub(crate) attention: AttentionState,
    pub(crate) active_tools: Vec<String>,
    pub(crate) stop_candidate: Option<StopDto>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompactionDto {
    pub(crate) prior_turn: TurnState,
    pub(crate) started_at: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChildCompactionDto {
    pub(crate) agent_id: String,
    pub(crate) phase: CompactionDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HistoryDto {
    pub(crate) entries: Vec<EventEnvelopeDto>,
}
