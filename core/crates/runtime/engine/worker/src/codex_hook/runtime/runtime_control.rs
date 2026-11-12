use super::CodexHookRuntime;
use crate::agent::codex::authority::CodexBindingAuthority;
use crate::agent::hooks::{InstrumentationState, ProcessState, HookProviderKind};
use crate::codex_hooks::CodexSnapshotLineage;
use crate::worker::codex_hook::{
    projection, publication::CodexAuthorityView, router::CodexRouteEntry,
};
use anyhow::{Result, ensure};

const INVALID: &str = "Codex control source is stale, unavailable, or superseded";
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../test_support/unit/codex_hook/control/control_registration_runtime.rs"]
mod control_test_support;

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn with_control_authority<T>(
        &self,
        entry: &CodexRouteEntry,
        authority: &CodexBindingAuthority,
        registry: &crate::agent::AgentRegistry,
        operation: impl FnOnce(&CodexSnapshotLineage, &dyn Fn() -> Result<()>) -> Result<T>,
    ) -> Result<T> {
        self.registration.with_control_authority(|lifecycle| {
            let owner = &entry.admission().record().request().identity;
            ensure!(&self.owner == owner && lifecycle.owner() == owner, INVALID);
            ensure!(
                matches!(
                    lifecycle.authority_view(),
                    Some(CodexAuthorityView::Active(_))
                ),
                INVALID
            );
            let reducer = lifecycle.reducer()?;
            ensure!(
                !reducer.is_revoked() && !reducer.authority_exhausted(),
                INVALID
            );
            let current = reducer.state();
            ensure!(
                current.expected_identity == *owner
                    && current.provider == HookProviderKind::Codex
                    && current.instrumentation == InstrumentationState::Authoritative
                    && current.process == ProcessState::Alive
                    && current.generation == authority.version.generation
                    && current.last_sequence.0 == authority.version.sequence
                    && reducer.revision() == authority.version.authority_revision,
                INVALID
            );
            let proof = lifecycle.prove_snapshot_current()?;
            projection::require_current_control_projection(
                entry, authority, current, reducer.revision(), registry,
            )?;
            let lineage = lifecycle.snapshot_lineage()?;
            let verify = || -> Result<()> {
                entry.admission().verify()?;
                proof.linearize_publication()
            };
            verify()?;
            // The installed registration, lifecycle mutex, and durable snapshot
            // proof remain live through this synchronous effect callback.
            operation(&lineage, &verify)
        })
    }
}
