use super::{
    binding::{self, ProjectionMode},
    route::ProjectionRoute,
};
use crate::agent::codex::authority::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::agent::hooks::NormalizedLifecycle;
use crate::agent::{AgentBinding, AgentRegistry, AgentState};
use crate::worker::WorkerState;
use crate::worker::codex_hook::router::CodexRouteEntry;
use anyhow::{Context, Result, ensure};
use yolo_agent_control_contract::{NativeTurnEnd, NativeTurnEndKind};

const STALE: &str = "Codex control projection does not match current source authority";

pub(in crate::worker::codex_hook) fn require_current_control_projection<'a>(entry: &CodexRouteEntry, authority: &CodexBindingAuthority, lifecycle: &'a NormalizedLifecycle, revision: u64, registry: &AgentRegistry) -> Result<Option<&'a str>> {
    let route = ProjectionRoute {
        revision: authority.version.route_revision,
        key: entry.key(),
        admission: entry.admission().clone(),
    };
    let mut projected = binding::project(&route, lifecycle, revision, ProjectionMode::Active, Some(&authority.binding), registry).context(STALE)?;
    let stop_turn = binding::stop_turn(lifecycle, &projected.binding);
    if stop_turn.is_some() && authority.binding.state == AgentState::WaitingForInput {
        projected.settle_stop(Some(&authority.binding));
    }
    ensure!(
        projected.status == CodexAuthorityStatus::Active
            && projected.version == authority.version
            && projected.provider == authority.provider_process()
            && projected.binding == authority.binding,
        STALE
    );
    Ok(stop_turn)
}

pub(in crate::worker) fn read_stop_turn_end(state: &WorkerState, binding: &AgentBinding, turn_id: &str) -> Option<NativeTurnEnd> {
    let probe = state.provider_runtime.get(&binding.kind)?.turn_end_probe()?;
    let end = probe.read_turn_end(binding).ok()??;
    (end.turn_id == turn_id).then_some(NativeTurnEnd {
        turn_id: end.turn_id,
        kind: match end.kind {
            yolo_agent_providers::TurnEndKind::Completed => NativeTurnEndKind::Completed,
            yolo_agent_providers::TurnEndKind::Aborted => NativeTurnEndKind::Aborted,
        },
    })
}
