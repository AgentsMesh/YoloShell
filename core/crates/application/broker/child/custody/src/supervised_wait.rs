use anyhow::Result;
use std::time::Instant;
use yolo_broker_supervision_state::SupervisionPaths;
use yolo_broker_supervision_state::{SupervisionPhase, SupervisionRecord};
use yolo_process::DetachedChildCustody;

const RECORD_REPLACED: &str = "broker supervision custody record changed unexpectedly";

pub(super) enum WaitOutcome {
    Reached,
    SupervisorExited,
    Deadline,
}

pub(super) fn wait(
    supervisor: &mut DetachedChildCustody,
    paths: &SupervisionPaths,
    expected: &SupervisionRecord,
    deadline: Instant,
    accepted: impl Fn(SupervisionPhase) -> bool,
) -> Result<WaitOutcome> {
    loop {
        if supervisor.try_wait()?.is_some() {
            return Ok(WaitOutcome::SupervisorExited);
        }
        let current = paths.read_record()?;
        anyhow::ensure!(expected.same_custody(&current), RECORD_REPLACED);
        if accepted(current.phase()) {
            return Ok(WaitOutcome::Reached);
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(WaitOutcome::Deadline);
        }
        std::thread::sleep(
            deadline
                .saturating_duration_since(now)
                .min(yolo_remote_broker::BROKER_READY_POLL_INTERVAL),
        );
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/supervised_wait_tests.rs"]
mod tests;
