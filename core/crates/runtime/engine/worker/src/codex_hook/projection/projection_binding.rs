use super::cursor::ProjectionCursor;
use super::route::ProjectionRoute;
use crate::agent::codex::authority::{CodexAuthorityStatus, CodexAuthorityVersion};
use crate::agent::hooks::{
    HookProviderKind, InstrumentationState, NormalizedLifecycle, ProcessIdentity, ProcessState,
    ProviderVersion,
};
use crate::agent::{
    AgentBinding, AgentKind, AgentRecoveryAvailability, AgentRegistry, AgentState, DetectionContext,
};
use yolo_agent_hook_generation::{ProjectionRules, project as project_state};

pub(super) struct ProjectedBinding {
    pub(super) binding: AgentBinding,
    pub(super) provider: ProcessIdentity,
    pub(super) provider_version: ProviderVersion,
    pub(super) status: CodexAuthorityStatus,
    pub(super) version: CodexAuthorityVersion,
}

pub(super) enum ProjectionMode {
    Active,
    Exhausted,
}

pub(super) fn project(
    route: &ProjectionRoute,
    lifecycle: &NormalizedLifecycle,
    authority_revision: u64,
    mode: ProjectionMode,
    previous: Option<&AgentBinding>, registry: &AgentRegistry,
) -> Option<ProjectedBinding> {
    let record = route.admission.record();
    let request = record.request();
    if route.admission.verify().is_err()
        || route.revision != record.route_revision()
        || route.key.runtime_epoch() != request.identity.runtime_epoch
        || route.key.invocation_id() != request.identity.invocation_id
        || lifecycle.expected_identity != request.identity
        || lifecycle.generation != request.identity.generation
        || lifecycle.provider != HookProviderKind::Codex
        || lifecycle.provider_version != request.provider_version
        || !valid_authority_revision(authority_revision)
    {
        return None;
    }
    let session_id = lifecycle.provider_session_id.as_ref()?;
    let cwd = lifecycle
        .current_cwd
        .clone()
        .unwrap_or_else(|| request.cwd.clone());
    let transcript = lifecycle.current_transcript_path.as_ref()?;
    let active = active_state(lifecycle);
    let (status, state, recovery) = match mode {
        ProjectionMode::Active => match active {
            Some(state) => (
                CodexAuthorityStatus::Active,
                state,
                AgentRecoveryAvailability::Available,
            ),
            None => (
                CodexAuthorityStatus::Unavailable,
                AgentState::Unknown,
                AgentRecoveryAvailability::Unavailable,
            ),
        },
        ProjectionMode::Exhausted => (
            CodexAuthorityStatus::Exhausted,
            AgentState::Unknown,
            AgentRecoveryAvailability::Unavailable,
        ),
    };
    let provider = request.identity.process;
    let process = (state != AgentState::Exited).then_some(provider);
    let now = crate::model::now_ms();
    let previous = previous.filter(|binding| {
        binding.kind == AgentKind::Codex && binding.provider_session_id == *session_id
    });
    let detected_at_ms = previous.map_or(now, |binding| binding.detected_at_ms);
    let updated_at_ms = previous.map_or(now.max(detected_at_ms), |binding| {
        now.max(detected_at_ms).max(binding.updated_at_ms)
    });
    let mut binding = AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: request.identity.terminal_id,
        kind: AgentKind::Codex,
        provider_session_id: session_id.clone(),
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        cwd: cwd.to_string_lossy().into_owned(),
        evidence_path: transcript.to_string_lossy().into_owned(),
        data_home: request.data_home.to_string_lossy().into_owned(),
        process_pid: process.map(|identity| identity.pid),
        process_start_time: process.map(|identity| identity.start_time),
        state,
        recovery_availability: recovery,
        detected_at_ms,
        updated_at_ms,
    };
    if let Some(previous) = previous.filter(|previous| same_projection(previous, &binding)) {
        binding.detected_at_ms = previous.detected_at_ms;
        binding.updated_at_ms = previous.updated_at_ms;
    }
    let context = DetectionContext::new(
        request.identity.terminal_id,
        request.cwd.clone(),
        [(AgentKind::Codex, request.data_home.clone())],
    );
    registry.validate_binding(&binding, &context).ok()?;
    Some(ProjectedBinding {
        binding,
        provider,
        provider_version: request.provider_version,
        status,
        version: ProjectionCursor::lifecycle_version(route, lifecycle, authority_revision),
    })
}

fn same_projection(left: &AgentBinding, right: &AgentBinding) -> bool {
    let mut normalized = left.clone();
    normalized.detected_at_ms = right.detected_at_ms;
    normalized.updated_at_ms = right.updated_at_ms;
    normalized == *right
}

pub(super) fn confirmed_exit(lifecycle: &NormalizedLifecycle) -> bool {
    lifecycle.instrumentation == InstrumentationState::Authoritative
        && lifecycle.process == ProcessState::Exited
        && lifecycle.final_hook_seen
}

fn active_state(lifecycle: &NormalizedLifecycle) -> Option<AgentState> {
    project_state(lifecycle, ProjectionRules::CODEX)
}

fn valid_authority_revision(revision: u64) -> bool {
    revision != u64::default() && revision != u64::MAX
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_binding_replay_tests.rs"]
mod replay_tests;
