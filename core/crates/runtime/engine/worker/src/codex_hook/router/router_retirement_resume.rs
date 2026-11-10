use super::state::{RouteState, RouterEntries};
use super::{
    CodexRetirementPermit, CodexRetirementReadiness, CodexRouteEntry, CodexRouterError,
    CodexSessionRouter,
};
use std::sync::Arc;

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn continue_retirement(
        &self,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<Option<CodexRetirementReadiness>, CodexRouterError> {
        expected.admission().verify()?;
        self.inner.continue_retirement(expected)
    }
}

impl RouterEntries {
    fn continue_retirement(
        self: &Arc<Self>,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<Option<CodexRetirementReadiness>, CodexRouterError> {
        let key = expected.key();
        let mut entries = self.lock()?;
        let Some(slot) = entries
            .get_mut(&key)
            .and_then(RouteState::active_mut)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
        else {
            return Ok(Some(CodexRetirementReadiness::Missing));
        };
        Ok(match slot.resume_retirement() {
            None => None,
            Some(false) => Some(CodexRetirementReadiness::Pending),
            Some(true) => Some(CodexRetirementReadiness::Ready(CodexRetirementPermit::new(
                Arc::clone(expected),
                Arc::clone(self),
            ))),
        })
    }
}
