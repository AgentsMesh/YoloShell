use crate::supervision_activation::ActivationSender;
use crate::supervisor_custody::SupervisorCustody;
use anyhow::Result;
use std::time::Instant;
use yolo_broker_child_custody::wait_for_custody_settlement;
use yolo_broker_supervision_state::SupervisionPaths;
use yolo_broker_supervision_state::{SupervisionPhase, SupervisionRecord};

const CLEANUP_DEADLINE_OVERFLOW: &str = "broker cleanup deadline overflow";

pub(super) fn terminate(
    custody: SupervisorCustody,
    control: Option<ActivationSender>,
    record: &mut SupervisionRecord,
    paths: &SupervisionPaths,
) -> Result<()> {
    if record.phase() == SupervisionPhase::Intent {
        return settle_unrecorded(custody, control);
    }
    let deadline = cleanup_deadline()?;
    let publication = record
        .begin_termination()
        .and_then(|()| paths.write_record(record));
    drop(control);
    let settlement = custody.settle();
    let reaped = wait_for_custody_settlement(deadline);
    let completion = combine(settlement, reaped);
    if completion.is_ok() {
        let publication_result = record
            .mark_settled()
            .and_then(|()| paths.write_record(record));
        return combine(publication, publication_result);
    }
    combine(publication, completion)
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/cleanup_phase.rs"]
mod tests;

pub(super) fn settle_unrecorded(
    custody: SupervisorCustody,
    control: Option<ActivationSender>,
) -> Result<()> {
    let deadline = cleanup_deadline()?;
    drop(control);
    let settlement = custody.settle();
    combine(settlement, wait_for_custody_settlement(deadline))
}

pub(super) fn after_error(
    custody: SupervisorCustody,
    control: Option<ActivationSender>,
    record: &mut SupervisionRecord,
    paths: &SupervisionPaths,
    primary: anyhow::Error,
) -> Result<()> {
    let cleanup = terminate(custody, control, record, paths);
    Err(yolo_broker_child_custody::combine_cleanup_failure(
        primary, cleanup,
    ))
}

fn cleanup_deadline() -> Result<Instant> {
    Instant::now()
        .checked_add(yolo_remote_broker::BROKER_READY_TIMEOUT)
        .ok_or_else(|| anyhow::anyhow!(CLEANUP_DEADLINE_OVERFLOW))
}

fn combine(primary: Result<()>, cleanup: Result<()>) -> Result<()> {
    match (primary, cleanup) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(primary), Err(cleanup)) => Err(yolo_broker_child_custody::combine_cleanup_failure(
            primary,
            Err(cleanup),
        )),
    }
}
