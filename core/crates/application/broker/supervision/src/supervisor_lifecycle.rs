use crate::supervision_activation::ActivationSender;
use anyhow::{Context, Result};
use std::process::ExitStatus;
use std::time::Instant;
use yolo_broker_supervision_state::SupervisionPaths;
use yolo_broker_supervision_state::{SupervisionPhase, SupervisionRecord};
use yolo_process::{DetachedChildCustody, ProcessIdentityObservation};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/supervisor_lifecycle_test_hook.rs"]
mod test_hook;

const READINESS_INSTANCE_MISSING: &str = "supervised broker readiness instance is missing";

pub(super) enum SupervisionOutcome {
    Reaped(ExitStatus),
    Terminate,
}

pub(super) fn supervise(
    child: &mut DetachedChildCustody,
    control: &mut ActivationSender,
    record: &mut SupervisionRecord,
    paths: &SupervisionPaths,
    deadline: Instant,
) -> Result<SupervisionOutcome> {
    loop {
        if let Some(outcome) = poll_supervision(child, control, record, paths, deadline)? {
            return Ok(outcome);
        }
        std::thread::sleep(yolo_remote_broker::BROKER_READY_POLL_INTERVAL);
    }
}

fn poll_supervision(
    child: &mut DetachedChildCustody,
    control: &mut ActivationSender,
    record: &mut SupervisionRecord,
    paths: &SupervisionPaths,
    deadline: Instant,
) -> Result<Option<SupervisionOutcome>> {
    if let Some(status) = child.try_wait()? {
        return Ok(Some(SupervisionOutcome::Reaped(status)));
    }
    if paths.termination_present(record)? {
        return Ok(Some(SupervisionOutcome::Terminate));
    }
    if record.phase() == SupervisionPhase::Ready && paths.adoption_present(record)? {
        let prepared = control.prepare_adoption(
            record.instance_id().context(READINESS_INSTANCE_MISSING)?,
            deadline,
        )?;
        record.mark_running()?;
        paths.write_record(record)?;
        #[cfg(test)]
        test_hook::after_running_persist();
        control.commit_adoption(prepared, deadline)?;
    }
    if record.phase() != SupervisionPhase::Running {
        let launcher = record.launcher_identity().observe();
        if launcher == ProcessIdentityObservation::Gone || Instant::now() >= deadline {
            return Ok(Some(SupervisionOutcome::Terminate));
        }
    }
    Ok(None)
}

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
#[path = "../integration_tests/supervisor_lifecycle_crash_tests.rs"]
mod crash_tests;
