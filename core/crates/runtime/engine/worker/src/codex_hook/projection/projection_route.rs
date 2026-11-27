use super::super::router::{CodexCommittedRoute, CodexRouteKey, DurableCodexAdmission};

#[derive(Clone)]
pub(super) struct ProjectionRoute {
    pub(super) revision: u64,
    pub(super) key: CodexRouteKey,
    pub(super) admission: DurableCodexAdmission,
}

impl ProjectionRoute {
    pub(super) fn capture(route: &CodexCommittedRoute) -> Self {
        Self {
            revision: route.revision,
            key: route.entry.key(),
            admission: route.entry.admission().clone(),
        }
    }
}
