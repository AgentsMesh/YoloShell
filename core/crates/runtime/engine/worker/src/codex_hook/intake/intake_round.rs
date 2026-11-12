use super::super::router::CodexSessionRouter;
use super::report::IntakeRoundReport;
use anyhow::{Context, Result};

pub(super) fn reconcile(router: &CodexSessionRouter) -> Result<IntakeRoundReport> {
    let mut entries = router
        .committed_entries()
        .map_err(anyhow::Error::new)
        .context(super::super::constants::CODEX_INTAKE_ENUMERATION_ERROR)?;
    entries
        .sort_unstable_by_key(|entry| (entry.key().runtime_epoch(), entry.key().invocation_id()));
    #[cfg(test)]
    round_test_gate::wait_after_enumeration(router);

    let mut aggregate = IntakeRoundReport::default();
    for entry in entries {
        match super::route::reconcile(router, &entry) {
            Ok(report) => aggregate.record_route(report),
            Err(error) => {
                tracing::warn!(
                    invocation_id = %entry.key().invocation_id(),
                    detail = %error,
                    "{}",
                    super::super::constants::CODEX_INTAKE_ROUTE_ERROR_LOG
                );
                aggregate.record_failure();
            }
        }
    }
    Ok(aggregate)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/intake/intake_round_test_gate.rs"]
mod round_test_gate;
