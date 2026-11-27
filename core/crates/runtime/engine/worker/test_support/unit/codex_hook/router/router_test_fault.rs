use super::state::RouteState;
use super::{
    CodexRouteEntry, CodexRouteKey, CodexRouteRetirement, CodexRouterError, CodexSessionRouter,
};
use crate::agent::hooks::HookIdentity;
use std::sync::Arc;
use std::time::Duration;

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn retire(
        &self,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<CodexRouteRetirement, CodexRouterError> {
        let key = expected.key();
        let mut entries = self.inner.lock()?;
        let Some(slot) = entries
            .get_mut(&key)
            .and_then(RouteState::active_mut)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
        else {
            return Ok(CodexRouteRetirement::Missing);
        };
        if slot.request_automatic_retirement_for_test() {
            entries.remove(&key);
            Ok(CodexRouteRetirement::Retired)
        } else {
            Ok(CodexRouteRetirement::Deferred)
        }
    }

    pub(in crate::worker::codex_hook) fn poison_lock_for_test(&self) {
        let entries = Arc::clone(&self.inner);
        let poisoned = std::thread::spawn(move || {
            let _guard = entries.lock().unwrap();
            std::panic::panic_any(());
        })
        .join();
        assert!(poisoned.is_err());
    }

    pub(in crate::worker::codex_hook) fn fail_next_settle_for_test(&self, error: CodexRouterError) {
        self.inner.fail_next_settle(error);
    }

    pub(in crate::worker::codex_hook) fn fail_next_abort_for_test(&self, error: CodexRouterError) {
        self.inner.fail_next_abort(error);
    }

    pub(super) fn wait_for_build_waiters_for_test(
        &self,
        identity: &HookIdentity,
        expected: usize,
        timeout: Duration,
    ) -> Result<bool, CodexRouterError> {
        let key = CodexRouteKey::from_identity(identity);
        self.inner
            .build_gate(key)?
            .wait_for_waiters(expected, timeout)
    }
}
