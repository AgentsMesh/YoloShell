use super::*;
use crate::agent::{AgentKind, AgentRecoveryAvailability};
use crate::claude_hooks::{
    ClaudeHookState, ClaudeLifecycleEvent, ClaudeSessionEndReason, ClaudeSessionStartSource,
};
use anyhow::{Context, Result, ensure};

pub(super) fn first(
    request: &ClaudeNativeRequest,
    worker: super::super::AuthorityProcessIdentity,
    boot_id: &str,
    evidence: &crate::claude::AuthoritativeClaudeEvidence,
) -> Result<ClaudeNativeGeneration> {
    ensure!(
        request.event_sequence == crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE
            && matches!(request.event, ClaudeLifecycleEvent::SessionStart { .. }),
        OUT_OF_ORDER
    );
    let info = evidence.evidence();
    let now = crate::model::now_ms();
    let mut record = ClaudeNativeGeneration {
        schema_version: SCHEMA_VERSION,
        scope: request.scope,
        worker,
        boot_id: boot_id.to_owned(),
        invocation_id: Uuid::new_v4(),
        revision: u64::default(),
        last_event_sequence: u64::default(),
        activity: AgentState::Unknown,
        final_session_end: false,
        revoked: false,
        binding: AgentBinding {
            schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
            terminal_id: request.scope.terminal_id,
            kind: AgentKind::ClaudeCode,
            executable: crate::claude::constants::EXECUTABLE.to_owned(),
            provider_session_id: info.provider_session_id.clone(),
            cwd: info.cwd.to_string_lossy().into_owned(),
            data_home: info.data_home.to_string_lossy().into_owned(),
            evidence_path: info.path.to_string_lossy().into_owned(),
            process_pid: Some(request.scope.agent.pid),
            process_start_time: Some(request.scope.agent.start_time),
            state: AgentState::Unknown,
            recovery_availability: AgentRecoveryAvailability::Available,
            detected_at_ms: now,
            updated_at_ms: now,
        },
        transcript_identity: evidence.identity(),
        cwd_identity: super::evidence::directory_identity(&info.cwd)?,
        data_home_identity: super::evidence::directory_identity(&info.data_home)?,
        replay_events: VecDeque::new(),
    };
    apply(&mut record, request, evidence)?;
    Ok(record)
}

pub(super) fn apply(
    record: &mut ClaudeNativeGeneration,
    request: &ClaudeNativeRequest,
    evidence: &crate::claude::AuthoritativeClaudeEvidence,
) -> Result<bool> {
    ensure!(!record.revoked, REVOKED);
    ensure!(record.scope == request.scope, INVALID);
    if let Some(previous) = record
        .replay_events
        .iter()
        .find(|item| item.event_sequence == request.event_sequence)
    {
        ensure!(previous == request, OUT_OF_ORDER);
        return Ok(false);
    }
    ensure!(
        record.last_event_sequence.checked_add(INCREMENT) == Some(request.event_sequence)
            && !record
                .replay_events
                .iter()
                .any(|prior| prior.event.event_id() == request.event.event_id()),
        OUT_OF_ORDER
    );
    validation::event(request, evidence)?;
    let info = evidence.evidence();
    ensure!(
        super::evidence::directory_identity(&info.data_home)? == record.data_home_identity,
        INVALID
    );
    if info.cwd == std::path::Path::new(&record.binding.cwd) {
        ensure!(
            super::evidence::directory_identity(&info.cwd)? == record.cwd_identity,
            INVALID
        );
    }
    let same_session = record.binding.provider_session_id == info.provider_session_id;
    if record.last_event_sequence != u64::default() && same_session {
        ensure!(
            record.transcript_identity == evidence.identity()
                && record.binding.evidence_path == info.path.to_string_lossy(),
            INVALID
        );
    }
    match &request.event {
        ClaudeLifecycleEvent::SessionStart { source, state, .. } => {
            ensure!(
                same_session
                    || matches!(
                        source,
                        ClaudeSessionStartSource::Clear
                            | ClaudeSessionStartSource::Resume
                            | ClaudeSessionStartSource::Fork
                    ),
                OUT_OF_ORDER
            );
            record
                .binding
                .provider_session_id
                .clone_from(&info.provider_session_id);
            record.activity = activity(*state);
            record.final_session_end = false;
        }
        ClaudeLifecycleEvent::StateChanged { state, .. } => {
            ensure!(same_session && active(record), OUT_OF_ORDER);
            record.activity = activity(*state);
        }
        ClaudeLifecycleEvent::CwdChanged { .. } => {
            ensure!(same_session && active(record), OUT_OF_ORDER);
        }
        ClaudeLifecycleEvent::SessionEnd { reason, .. } => {
            ensure!(same_session, OUT_OF_ORDER);
            record.activity = AgentState::Unknown;
            record.final_session_end = *reason == ClaudeSessionEndReason::Other;
        }
        ClaudeLifecycleEvent::InteractionRequested { .. } => {
            anyhow::bail!("interaction requests are not lifecycle events")
        }
    }
    record.binding.cwd = info.cwd.to_string_lossy().into_owned();
    record.cwd_identity = super::evidence::directory_identity(&info.cwd)?;
    record.binding.evidence_path = info.path.to_string_lossy().into_owned();
    record.transcript_identity = evidence.identity();
    record.binding.state = record.activity;
    record.binding.updated_at_ms = crate::model::now_ms().max(record.binding.updated_at_ms);
    if !same_session {
        record.binding.detected_at_ms = record.binding.updated_at_ms;
    }
    record.last_event_sequence = request.event_sequence;
    record.revision = record.revision.checked_add(INCREMENT).context(CAPACITY)?;
    if record.replay_events.len() == super::super::constants::MAX_REPLAY_EVENT_IDS {
        record.replay_events.pop_front();
    }
    record.replay_events.push_back(request.clone());
    Ok(true)
}

fn activity(state: ClaudeHookState) -> AgentState {
    match state {
        ClaudeHookState::Running => AgentState::Running,
        ClaudeHookState::WaitingForInput => AgentState::WaitingForInput,
    }
}

fn active(record: &ClaudeNativeGeneration) -> bool {
    !record.final_session_end
        && matches!(
            record.activity,
            AgentState::Running | AgentState::WaitingForInput
        )
}
