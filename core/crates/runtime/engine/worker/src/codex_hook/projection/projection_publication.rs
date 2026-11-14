use super::binding::{self, ProjectionMode};
use super::cursor;
use super::route::ProjectionRoute;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::agent::hooks::NormalizedLifecycle;
use crate::agent::{AgentBinding, AgentBindingStore, AgentKind, AgentState};
use crate::model::RuntimeRecord;
use crate::worker::{WorkerEvent, WorkerState, agent_monitor_publication_fence, lock_recover};
use anyhow::{Result, ensure};
use std::cmp::Ordering;
use std::time::Duration;

const STOP_SETTLEMENT_ATTEMPTS: usize = 5;
const STOP_SETTLEMENT_DELAY: Duration = Duration::from_millis(20);
#[path = "projection_authority.rs"]
mod authority;

pub(super) async fn publish_lifecycle(
    state: &WorkerState,
    route: &ProjectionRoute,
    lifecycle: &NormalizedLifecycle,
    authority_revision: u64,
    mode: ProjectionMode,
) -> Result<bool> {
    let _gate = state.agent_generation_gate.lock().await;
    let expected = lock_recover(&state.record).clone();
    let publication_fence = agent_monitor_publication_fence::acquire(state, &expected)?;
    let store = binding_store(state);
    let before = store.read_raw_in_namespace(state.namespace.lease())?;
    let authority_store = store.codex_authority_store()?;
    let current = authority_store.load_in_namespace(state.namespace.lease())?;
    let version = cursor::ProjectionCursor::lifecycle_version(route, lifecycle, authority_revision);
    let previous = replay_base(current.as_ref(), version, before.as_ref());
    let Some(mut projected) = binding::project(
        route, lifecycle, authority_revision, mode, previous, state.agent_registry.as_ref(),
    ) else {
        return Ok(false);
    };
    if projected.binding.state == AgentState::Active
        && let Some(stop) = lifecycle.stop_candidate.as_ref()
        && let Some(turn_id) = stop.turn_id.as_deref()
        && let Some(probe) = state.provider_runtime.get(&projected.binding.kind).and_then(|runtime| runtime.turn_end_probe())
    {
        for _ in 0..STOP_SETTLEMENT_ATTEMPTS {
            if matches!(probe.read_turn_end(&projected.binding), Ok(Some(end)) if end.turn_id == turn_id) {
                projected.binding.state = AgentState::WaitingForInput;
                break;
            }
            tokio::time::sleep(STOP_SETTLEMENT_DELAY).await;
        }
    }
    let runtime = runtime_identity(&expected);
    let mut authority = authority::create(&projected, runtime, &expected.boot_id)?;
    if authority.status != CodexAuthorityStatus::Active
        && let Some(current) = current.as_ref().filter(|current| {
            current.matches_runtime(runtime)
                && current.provider_process() == authority.provider_process()
                && current.version.route_revision == authority.version.route_revision
                && current.version.invocation_id == authority.version.invocation_id
        })
    {
        authority = authority.inherit_verified_resume_identity(current)?;
    }
    if publication_is_blocked(current.as_ref(), &authority) {
        return Ok(true);
    }
    authority_store.publish_in_namespace_with_commit_fence(
        state.namespace.lease(),
        store.path(),
        &authority,
        || publication_fence.verify(),
    )?;
    ensure!(
        store
            .read_raw_in_namespace(state.namespace.lease())?
            .as_ref()
            == Some(&projected.binding),
        super::super::constants::CODEX_PROJECTION_BINDING_CHANGED
    );
    publish_event(state, before.as_ref(), projected.binding);
    Ok(true)
}

pub(super) async fn invalidate(state: &WorkerState) -> Result<bool> {
    let _gate = state.agent_generation_gate.lock().await;
    let expected = lock_recover(&state.record).clone();
    let publication_fence = agent_monitor_publication_fence::acquire(state, &expected)?;
    let store = binding_store(state);
    let before = store.read_raw_in_namespace(state.namespace.lease())?;
    let authority = store
        .codex_authority_store()?
        .invalidate_in_namespace_with_commit_fence(
            state.namespace.lease(),
            store.path(),
            runtime_identity(&expected),
            || publication_fence.verify(),
        )?;
    let Some(authority) = authority else {
        return Ok(false);
    };
    ensure!(
        store
            .read_raw_in_namespace(state.namespace.lease())?
            .as_ref()
            == Some(&authority.binding),
        super::super::constants::CODEX_PROJECTION_BINDING_CHANGED
    );
    publish_event(state, before.as_ref(), authority.binding);
    Ok(true)
}

fn publication_is_blocked(
    current: Option<&CodexBindingAuthority>,
    next: &CodexBindingAuthority,
) -> bool {
    let Some(current) = current else {
        return false;
    };
    if current.status == CodexAuthorityStatus::Revoked
        && next.version.route_revision <= current.version.route_revision
    {
        return true;
    }
    match cursor::compare(current.version, next.version) {
        Some(Ordering::Greater) | None => true,
        Some(Ordering::Equal) => {
            next.status < current.status && !next.is_canonical_reactivation_of(current)
        }
        Some(Ordering::Less) => false,
    }
}

fn replay_base<'a>(
    current: Option<&'a CodexBindingAuthority>,
    version: crate::agent::codex::authority::CodexAuthorityVersion,
    fallback: Option<&'a AgentBinding>,
) -> Option<&'a AgentBinding> {
    current
        .filter(|authority| authority.version == version)
        .map(|authority| &authority.binding)
        .or(fallback)
}

fn runtime_identity(record: &RuntimeRecord) -> AuthorityRuntimeIdentity {
    AuthorityRuntimeIdentity {
        runtime_epoch: record.runtime_epoch,
        worker: AuthorityProcessIdentity {
            pid: record.worker_pid,
            start_time: record.worker_start_time,
        },
    }
}

fn binding_store(state: &WorkerState) -> AgentBindingStore {
    AgentBindingStore::with_provider_runtime(
        state.paths.agent_binding(state.terminal_id()),
        std::sync::Arc::clone(&state.provider_runtime),
    )
}

fn publish_event(state: &WorkerState, before: Option<&AgentBinding>, after: AgentBinding) {
    if before == Some(&after) {
        return;
    }
    let event_type = if before.is_some_and(|binding| same_subject(binding, &after)) {
        crate::protocol::event::AGENT_STATUS_CHANGED
    } else {
        crate::protocol::event::AGENT_BINDING_CHANGED
    };
    state.publish_agent_event(event_type, after);
}

fn same_subject(left: &AgentBinding, right: &AgentBinding) -> bool {
    left.kind == AgentKind::Codex
        && right.kind == AgentKind::Codex
        && left.terminal_id == right.terminal_id
        && left.provider_session_id == right.provider_session_id
        && left.executable == right.executable
        && left.cwd == right.cwd
        && left.evidence_path == right.evidence_path
        && left.data_home == right.data_home
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_publication_order_tests.rs"]
mod order_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_publication_revocation_tests.rs"]
mod revocation_tests;
