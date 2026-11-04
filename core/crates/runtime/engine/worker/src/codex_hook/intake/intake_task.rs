use super::blocking_custody::BlockingSettlementCustody;
use super::cancellation::IngressCancellation;
use super::router::CodexSessionRouter;
use anyhow::{Context, Result};
use std::sync::Arc;

#[path = "intake_report.rs"]
mod report;
#[path = "intake_retirement_coordinator.rs"]
mod retirement;
#[path = "intake_round.rs"]
mod round;
#[path = "intake_route_round.rs"]
mod route;

#[cfg(test)]
pub(in crate::worker::codex_hook) use retirement::{RetirementFaultStage, clear_test_faults};

#[cfg(test)]
pub(in crate::worker::codex_hook) fn reconcile_round_for_test(
    router: &CodexSessionRouter,
) -> Result<report::IntakeRoundReport> {
    round::reconcile(router)
}

#[cfg(test)]
pub(in crate::worker::codex_hook) fn reconcile_route_for_test(
    router: &CodexSessionRouter,
    entry: &Arc<super::router::CodexRouteEntry>,
) -> Result<()> {
    route::reconcile(router, entry).map(drop)
}

pub(super) fn reconcile_recovered_retirement(
    readiness: super::router::CodexRetirementReadiness,
) -> Result<()> {
    retirement::reconcile(readiness, None).map(drop)
}

pub(super) async fn run(
    router: Arc<CodexSessionRouter>,
    cancellation: IngressCancellation,
    settlements: BlockingSettlementCustody,
) -> Result<()> {
    let mut retry = tokio::time::interval(super::constants::CODEX_INTAKE_RETRY_INTERVAL);
    retry.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Ok(()),
            _ = retry.tick() => reconcile(Arc::clone(&router), settlements.clone()).await?,
        }
    }
}

pub(super) async fn reconcile(
    router: Arc<CodexSessionRouter>,
    settlements: BlockingSettlementCustody,
) -> Result<()> {
    let task = tokio::task::spawn_blocking(move || {
        let _settlement = settlements.admit();
        round::reconcile(router.as_ref())
    });
    let report = task
        .await
        .map_err(anyhow::Error::new)
        .context(super::constants::CODEX_INTAKE_TASK_ERROR)?
        .context(super::constants::CODEX_INTAKE_TASK_ERROR)?;
    report.log();
    Ok(())
}
