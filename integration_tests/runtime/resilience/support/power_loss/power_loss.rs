use super::runtime_record_authority::BoundRuntimeRecord;
use super::*;

#[path = "power_loss_barrier.rs"]
mod barrier;
#[cfg(test)]
#[path = "power_loss_guardian_pause.rs"]
mod guardian_pause;
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
#[path = "power_loss_guardians.rs"]
mod guardians;
#[cfg(target_os = "linux")]
#[path = "power_loss_custody_linux.rs"]
mod guardians;
#[cfg(not(any(
    target_os = "linux",
    all(target_os = "macos", feature = "adversarial-test-hooks")
)))]
#[path = "power_loss_guardians_unsupported.rs"]
mod guardians;
#[cfg(all(
    test,
    any(
        target_os = "linux",
        all(target_os = "macos", feature = "adversarial-test-hooks")
    )
))]
#[path = "power_loss_guardians_unsupported.rs"]
mod guardians_unsupported;
#[path = "power_loss_validation.rs"]
mod validation;
#[path = "power_loss_workers.rs"]
mod workers;

const GUARDIAN_CAPTURE_CONTEXT: &str = "bind simulated power-loss Guardians";
const BARRIER_CAPTURE_CONTEXT: &str = "capture simulated power-loss freeze barrier";
const BARRIER_FREEZE_CONTEXT: &str = "freeze complete simulated power-loss process closure";
const BARRIER_KILL_CONTEXT: &str = "kill complete simulated power-loss process closure";
const BARRIER_GONE_CONTEXT: &str = "confirm simulated power-loss process closure is gone";
const RUNTIME_VALIDATE_CONTEXT: &str = "validate exact simulated power-loss runtime record";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GuardianExpectation {
    RequiredOnSupportedPlatform,
    ExplicitlyAbsentFixture,
}

pub(crate) async fn simulate_power_loss(harness: &mut Harness, candidates: &[RuntimeRecord]) {
    let paths = harness.paths();
    let records = harness.bind_cleanup_records(candidates);
    let barrier = prepare_bound_power_loss(
        &paths,
        &records,
        GuardianExpectation::RequiredOnSupportedPlatform,
    )
    .await;
    harness.kill_host_hard();
    finish_bound_power_loss(&paths, &records, barrier).await;
}

async fn simulate_bound_power_loss(
    paths: &Paths,
    records: &[BoundRuntimeRecord],
    guardian_expectation: GuardianExpectation,
) {
    let barrier = prepare_bound_power_loss(paths, records, guardian_expectation).await;
    finish_bound_power_loss(paths, records, barrier).await;
}

async fn prepare_bound_power_loss(
    paths: &Paths,
    records: &[BoundRuntimeRecord],
    guardian_expectation: GuardianExpectation,
) -> barrier::PowerLossFreezeBarrier {
    validation::validate_targets(paths, records).expect(RUNTIME_VALIDATE_CONTEXT);
    validation::assert_exit_evidence_absent(paths, records);
    validation::assert_targets_live(records);
    let guardians =
        guardians::capture(paths, records, guardian_expectation).expect(GUARDIAN_CAPTURE_CONTEXT);
    let mut barrier = barrier::PowerLossFreezeBarrier::capture(paths, records, guardians)
        .await
        .expect(BARRIER_CAPTURE_CONTEXT);
    barrier
        .freeze_all(paths)
        .await
        .expect(BARRIER_FREEZE_CONTEXT);
    validation::validate_targets(paths, records).expect(RUNTIME_VALIDATE_CONTEXT);
    validation::assert_exit_evidence_absent(paths, records);
    barrier
}

async fn finish_bound_power_loss(
    paths: &Paths,
    records: &[BoundRuntimeRecord],
    mut barrier: barrier::PowerLossFreezeBarrier,
) {
    barrier
        .kill_all(paths, records)
        .await
        .expect(BARRIER_KILL_CONTEXT);
    barrier
        .wait_for_gone(paths, records)
        .await
        .expect(BARRIER_GONE_CONTEXT);
    validation::assert_exit_evidence_absent(paths, records);
}

fn runtime_child(record: &BoundRuntimeRecord) -> yolo_process_test_support::ProcessIdentity {
    cleanup_tree::runtime_child_identity(record.record())
}

fn runtime_worker(record: &BoundRuntimeRecord) -> yolo_process_test_support::ProcessIdentity {
    cleanup_tree::runtime_worker_identity(record.record())
}

#[cfg(test)]
#[path = "power_loss_process_tree_tests.rs"]
mod process_tree_tests;
#[cfg(test)]
#[path = "power_loss_tests.rs"]
mod tests;
