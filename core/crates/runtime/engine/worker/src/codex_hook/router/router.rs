#[path = "router_admission.rs"]
mod admission;
#[path = "router_build_gate.rs"]
mod build_gate;
#[path = "router_constants.rs"]
mod constants;
#[path = "router_control.rs"]
mod control;
#[path = "router_entry.rs"]
mod entry;
#[path = "router_error.rs"]
mod error;
#[path = "router_install.rs"]
mod install;
#[path = "router_install_state.rs"]
mod install_state;
#[path = "router_key.rs"]
mod key;
#[path = "router_lease.rs"]
mod lease;
#[path = "router_lifecycle.rs"]
mod lifecycle;
#[path = "router_publication.rs"]
mod publication;
#[path = "router_recovery.rs"]
mod recovery;
#[path = "router_retirement_resume.rs"]
mod retirement_resume;
#[path = "router_slot_retirement.rs"]
mod slot_retirement;
#[path = "router_state.rs"]
mod state;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/router/router_test_fault.rs"]
mod test_fault;
pub(in crate::worker::codex_hook) use admission::DurableCodexAdmission;
pub(in crate::worker::codex_hook) use constants::MAX_ACTIVE_CODEX_INVOCATIONS;
pub(in crate::worker::codex_hook) use entry::CodexRouteEntry;
pub(in crate::worker::codex_hook) use error::CodexRouterError;
pub(in crate::worker::codex_hook) use key::CodexRouteKey;
pub(in crate::worker::codex_hook) use publication::{CodexCommittedRoute, CodexRouteSubscription};
pub(in crate::worker::codex_hook) use recovery::CodexRecoveredRoute;

#[cfg(test)]
pub(in crate::worker::codex_hook) type RouterTestKey = Uuid;

use super::runtime::CodexHookRuntime;
use state::RouterEntries;
use std::sync::Arc;
use uuid::Uuid;
pub(in crate::worker::codex_hook) struct CodexSessionRouter {
    paths: crate::paths::Paths,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    capacity_limit: usize,
    inner: Arc<RouterEntries>,
    #[cfg(test)]
    test_key: RouterTestKey,
}

#[must_use]
pub(in crate::worker::codex_hook) struct CodexRouteLease {
    entry: Arc<CodexRouteEntry>,
    router: Arc<RouterEntries>,
    replay: bool,
    released: bool,
}

#[must_use]
pub(in crate::worker::codex_hook) struct CodexRetirementPermit {
    entry: Arc<CodexRouteEntry>,
    router: Arc<RouterEntries>,
    released: bool,
}

#[must_use]
pub(in crate::worker::codex_hook) enum CodexRetirementReadiness {
    Ready(CodexRetirementPermit),
    Pending,
    Missing,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::worker::codex_hook) enum CodexRouteRetirement {
    Retired,
    Deferred,
    Missing,
}

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn new(
        paths: crate::paths::Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Result<Self, CodexRouterError> {
        Self::build(
            paths,
            terminal_id,
            runtime_epoch,
            MAX_ACTIVE_CODEX_INVOCATIONS,
        )
    }

    fn build(
        paths: crate::paths::Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        capacity_limit: usize,
    ) -> Result<Self, CodexRouterError> {
        if terminal_id.is_nil() || runtime_epoch.is_nil() {
            return Err(CodexRouterError::Scope);
        }
        if capacity_limit == usize::default() {
            return Err(CodexRouterError::Capacity);
        }
        Ok(Self {
            paths,
            terminal_id,
            runtime_epoch,
            capacity_limit,
            inner: Arc::new(RouterEntries::default()),
            #[cfg(test)]
            test_key: RouterTestKey::new_v4(),
        })
    }

    #[cfg(test)]
    pub(super) fn with_capacity_limit(
        paths: crate::paths::Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        capacity_limit: usize,
    ) -> Result<Self, CodexRouterError> {
        Self::build(paths, terminal_id, runtime_epoch, capacity_limit)
    }

    pub(in crate::worker::codex_hook) fn terminal_id(&self) -> Uuid {
        self.terminal_id
    }

    pub(in crate::worker::codex_hook) fn runtime_epoch(&self) -> Uuid {
        self.runtime_epoch
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> Result<usize, CodexRouterError> {
        Ok(self.inner.lock()?.len())
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) fn test_key(&self) -> RouterTestKey {
        self.test_key
    }
}

#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/codex_router.rs"]
pub(in crate::worker::codex_hook) mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/router/router_tests.rs"]
pub(in crate::worker::codex_hook) mod tests;
