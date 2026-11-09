pub(crate) mod capacity;

use crate::event::{EventHistory, EventIds};
use crate::identity::{EventSequence, Generation, HookIdentity, HookProviderKind, ProviderVersion};
use std::collections::BTreeMap;
pub use yolo_hooks::state::{
    AttentionState, ChildLifecycle, InstrumentationState, ProcessState, StopCandidate,
    TerminationCandidate, ToolSet, TurnState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompactionPhase {
    prior_turn: TurnState,
    started_at: EventSequence,
}

impl CompactionPhase {
    pub(crate) fn begin(prior_turn: TurnState, started_at: EventSequence) -> Self {
        Self {
            prior_turn,
            started_at,
        }
    }

    pub(crate) fn admits_completion(&self, sequence: EventSequence) -> bool {
        sequence > self.started_at
    }

    pub(crate) fn prior_turn(self) -> TurnState {
        self.prior_turn
    }
    pub(crate) fn started_at(self) -> EventSequence {
        self.started_at
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedLifecycle {
    pub provider: HookProviderKind,
    pub provider_version: ProviderVersion,
    pub process: ProcessState,
    pub turn: TurnState,
    pub attention: AttentionState,
    pub instrumentation: InstrumentationState,
    pub provider_session_id: Option<String>,
    pub current_cwd: Option<std::path::PathBuf>,
    pub current_transcript_path: Option<std::path::PathBuf>,
    pub generation: Generation,
    pub last_sequence: EventSequence,
    pub stop_candidate: Option<StopCandidate>,
    pub termination_candidate: Option<TerminationCandidate>,
    pub final_hook_seen: bool,
    pub active_tools: ToolSet,
    pub children: BTreeMap<String, ChildLifecycle>,
    pub expected_identity: HookIdentity,
    pub history: EventHistory,
    pub event_ids: EventIds,
    pub compaction: Option<CompactionPhase>,
    pub child_compactions: BTreeMap<String, CompactionPhase>,
    pub revoked: bool,
    pub revocation: Option<crate::revocation::RevocationEvidence>,
}

impl NormalizedLifecycle {
    pub(crate) fn new(
        expected_identity: HookIdentity,
        provider: HookProviderKind,
        provider_version: ProviderVersion,
    ) -> Self {
        Self {
            provider,
            provider_version,
            process: ProcessState::Alive,
            turn: TurnState::Unknown,
            attention: AttentionState::None,
            instrumentation: InstrumentationState::Pending,
            provider_session_id: None,
            current_cwd: None,
            current_transcript_path: None,
            generation: expected_identity.generation,
            last_sequence: EventSequence::ZERO,
            stop_candidate: None,
            termination_candidate: None,
            final_hook_seen: false,
            active_tools: ToolSet::new(),
            children: BTreeMap::new(),
            expected_identity,
            history: EventHistory::new(),
            event_ids: EventIds::new(),
            compaction: None,
            child_compactions: BTreeMap::new(),
            revoked: false,
            revocation: None,
        }
    }
}
