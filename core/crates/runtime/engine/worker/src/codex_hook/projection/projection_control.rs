use super::{
    binding::{self, ProjectionMode},
    route::ProjectionRoute,
};
use crate::agent::codex::authority::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::agent::hooks::NormalizedLifecycle;
use crate::agent::AgentRegistry;
use crate::worker::codex_hook::router::CodexRouteEntry;
use anyhow::{Context, Result, ensure};

const STALE: &str = "Codex control projection does not match current source authority";

pub(in crate::worker::codex_hook) fn require_current_control_projection(
    entry: &CodexRouteEntry,
    authority: &CodexBindingAuthority,
    lifecycle: &NormalizedLifecycle,
    revision: u64,
    registry: &AgentRegistry,
) -> Result<()> {
    let route = ProjectionRoute {
        revision: authority.version.route_revision,
        key: entry.key(),
        admission: entry.admission().clone(),
    };
    let projected = binding::project(
        &route, lifecycle, revision, ProjectionMode::Active, Some(&authority.binding), registry,
    )
    .context(STALE)?;
    ensure!(
        projected.status == CodexAuthorityStatus::Active
            && projected.version == authority.version
            && projected.provider == authority.provider_process()
            && projected.binding == authority.binding,
        STALE
    );
    Ok(())
}
