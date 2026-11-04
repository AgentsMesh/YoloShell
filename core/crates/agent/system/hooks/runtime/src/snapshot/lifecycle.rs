use crate::event::{EventHistory, EventIds, ToolSet};
use crate::identity::{EventSequence, HookIdentity, HookProviderKind, ProviderVersion};
use crate::snapshot::SnapshotViolation;
use crate::snapshot::wire::{
    ChildCompactionDto, ChildDto, ChildEntryDto, CompactionDto, LifecycleDto, StopDto,
};
use crate::state::{ChildLifecycle, CompactionPhase, NormalizedLifecycle, StopCandidate};
use std::collections::BTreeMap;

pub(crate) fn lifecycle_to_dto(state: &NormalizedLifecycle) -> LifecycleDto {
    LifecycleDto {
        process: state.process,
        turn: state.turn,
        attention: state.attention,
        instrumentation: state.instrumentation,
        provider_session_id: state.provider_session_id.clone(),
        current_cwd: state.current_cwd.clone(),
        current_transcript_path: state.current_transcript_path.clone(),
        last_sequence: state.last_sequence.0,
        stop_candidate: state.stop_candidate.as_ref().map(StopDto::from),
        termination_candidate: state.termination_candidate,
        final_hook_seen: state.final_hook_seen,
        active_tools: state.active_tools.iter().cloned().collect(),
        children: state
            .children
            .iter()
            .map(|(agent_id, lifecycle)| ChildEntryDto {
                agent_id: agent_id.clone(),
                lifecycle: ChildDto::from(lifecycle),
            })
            .collect(),
        compaction: state.compaction.map(CompactionDto::from),
        child_compactions: state
            .child_compactions
            .iter()
            .map(|(agent_id, phase)| ChildCompactionDto {
                agent_id: agent_id.clone(),
                phase: CompactionDto::from(*phase),
            })
            .collect(),
        revoked: state.revoked,
        revocation: crate::snapshot::revocation::convert::to_dto(state.revocation.as_ref()),
    }
}

pub(crate) fn lifecycle_from_dto(
    dto: LifecycleDto,
    expected_identity: HookIdentity,
    provider: HookProviderKind,
    provider_version: ProviderVersion,
    history: EventHistory,
    event_ids: EventIds,
) -> Result<NormalizedLifecycle, SnapshotViolation> {
    let revocation = crate::snapshot::revocation::convert::from_dto(
        dto.revocation,
        &expected_identity,
        &provider,
        provider_version,
    )?;
    let active_tools = tool_set(dto.active_tools)?;
    let mut children = BTreeMap::new();
    for entry in dto.children {
        let agent_id = entry.agent_id;
        let lifecycle = child_from_dto(entry.lifecycle)?;
        if children.insert(agent_id, lifecycle).is_some() {
            return Err(SnapshotViolation::InvalidState);
        }
    }
    let mut child_compactions = BTreeMap::new();
    for entry in dto.child_compactions {
        if child_compactions
            .insert(entry.agent_id, CompactionPhase::from(entry.phase))
            .is_some()
        {
            return Err(SnapshotViolation::InvalidState);
        }
    }
    Ok(NormalizedLifecycle {
        provider,
        provider_version,
        process: dto.process,
        turn: dto.turn,
        attention: dto.attention,
        instrumentation: dto.instrumentation,
        provider_session_id: dto.provider_session_id,
        current_cwd: dto.current_cwd,
        current_transcript_path: dto.current_transcript_path,
        generation: expected_identity.generation,
        last_sequence: EventSequence(dto.last_sequence),
        stop_candidate: dto.stop_candidate.map(StopCandidate::from),
        termination_candidate: dto.termination_candidate,
        final_hook_seen: dto.final_hook_seen,
        active_tools,
        children,
        expected_identity,
        history,
        event_ids,
        compaction: dto.compaction.map(CompactionPhase::from),
        child_compactions,
        revoked: dto.revoked,
        revocation,
    })
}

fn child_from_dto(child: ChildDto) -> Result<ChildLifecycle, SnapshotViolation> {
    Ok(ChildLifecycle {
        active: child.active,
        turn: child.turn,
        attention: child.attention,
        active_tools: tool_set(child.active_tools)?,
        stop_candidate: child.stop_candidate.map(StopCandidate::from),
    })
}

fn tool_set(tools: Vec<String>) -> Result<ToolSet, SnapshotViolation> {
    let count = tools.len();
    let tools = tools.into_iter().collect::<ToolSet>();
    if tools.len() != count {
        return Err(SnapshotViolation::InvalidState);
    }
    Ok(tools)
}

impl From<&StopCandidate> for StopDto {
    fn from(stop: &StopCandidate) -> Self {
        Self {
            event_id: stop.event_id,
            turn_id: stop.turn_id.clone(),
            continued: stop.continued,
        }
    }
}

impl From<StopDto> for StopCandidate {
    fn from(stop: StopDto) -> Self {
        Self {
            event_id: stop.event_id,
            turn_id: stop.turn_id,
            continued: stop.continued,
        }
    }
}

impl From<&ChildLifecycle> for ChildDto {
    fn from(child: &ChildLifecycle) -> Self {
        Self {
            active: child.active,
            turn: child.turn,
            attention: child.attention,
            active_tools: child.active_tools.iter().cloned().collect(),
            stop_candidate: child.stop_candidate.as_ref().map(StopDto::from),
        }
    }
}

impl From<CompactionPhase> for CompactionDto {
    fn from(phase: CompactionPhase) -> Self {
        Self {
            prior_turn: phase.prior_turn(),
            started_at: phase.started_at().0,
        }
    }
}

impl From<CompactionDto> for CompactionPhase {
    fn from(phase: CompactionDto) -> Self {
        Self::begin(phase.prior_turn, EventSequence(phase.started_at))
    }
}
