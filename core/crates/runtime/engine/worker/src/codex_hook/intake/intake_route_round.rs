use super::super::router::{CodexRouteEntry, CodexSessionRouter};
use super::report::IntakeRouteReport;
use crate::process::ExactProcessGone;
use anyhow::Result;
use std::sync::Arc;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/intake/intake_route_test_probe.rs"]
mod test_probe;

pub(super) fn reconcile(
    router: &CodexSessionRouter,
    entry: &Arc<CodexRouteEntry>,
) -> Result<IntakeRouteReport> {
    let gone = capture_process_gone(router, entry);
    if let Some(readiness) = router
        .continue_retirement(entry)
        .map_err(anyhow::Error::new)?
    {
        return super::retirement::reconcile(readiness, gone);
    }
    let Some(gone) = gone else {
        return entry
            .runtime()
            .reconcile_routed_intake(router)
            .map(IntakeRouteReport::Active);
    };
    let readiness = router.begin_retirement(entry).map_err(anyhow::Error::new)?;
    super::retirement::reconcile(readiness, Some(gone))
}

fn process_identity(entry: &CodexRouteEntry) -> crate::process::ProcessIdentity {
    let process = entry.admission().record().request().identity.process;
    crate::process::ProcessIdentity {
        pid: process.pid,
        start_time: process.start_time,
    }
}

fn capture_process_gone(
    _router: &CodexSessionRouter,
    entry: &Arc<CodexRouteEntry>,
) -> Option<ExactProcessGone> {
    let identity = process_identity(entry);
    #[cfg(test)]
    if let Some(observation) = test_probe::take(_router, entry) {
        return ExactProcessGone::capture_for_test(identity, observation);
    }
    ExactProcessGone::capture(identity)
}
