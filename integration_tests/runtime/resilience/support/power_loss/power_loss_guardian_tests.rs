use super::super::super::runtime_record_test_support::{bind_record, record_for_identity};
use super::*;
use futures::FutureExt;

#[path = "power_loss_guardian_snapshot_lock_tests.rs"]
mod snapshot_lock_tests;

const TEST_WORKER_PID: u32 = 41_001;
const TEST_WORKER_START_TIME: u64 = 51_001;
const TEST_CHILD_PID: u32 = 41_002;
const TEST_CHILD_START_TIME: u64 = 51_002;
const STALE_GENERATION_INCREMENT: u64 = 1;
const GUARDIAN_SURVIVED_DETAIL: &str = "detached Guardian survived simulated power loss";
const EXACT_GUARDIAN_COUNT_DETAIL: &str = "session did not expose exactly one Guardian";
const GUARDIAN_GONE_TIMEOUT_DETAIL: &str = "detached Guardian did not exit";
const POWER_LOSS_FAILURE_LOG_LABEL: &str = "simulated power-loss Host and Guardian stderr";
const GUARDIAN_BINDING_SESSION_NAME: &str = "power-loss-guardian-binding";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simulated_power_loss_terminates_the_detached_guardian() {
    let mut harness = Harness::start().await;
    let session = harness.create_cat("power-loss-guardian").await;
    let terminal_id = session.manifest.terminal_id;
    let paths = harness.paths();
    let record: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let records = harness.bind_cleanup_records(std::slice::from_ref(&record));
    let guardians = capture(
        &paths,
        &records,
        GuardianExpectation::RequiredOnSupportedPlatform,
    )
    .unwrap();
    let [guardian] = guardians.as_slice() else {
        panic!("{EXACT_GUARDIAN_COUNT_DETAIL}: {}", guardians.len());
    };
    let guardian = guardian.identity();

    let outcome = std::panic::AssertUnwindSafe(simulate_power_loss(
        &mut harness,
        std::slice::from_ref(&record),
    ))
    .catch_unwind()
    .await;
    if let Err(payload) = outcome {
        eprintln!(
            "{POWER_LOSS_FAILURE_LOG_LABEL}: {}",
            harness.host_stderr_tail()
        );
        std::panic::resume_unwind(payload);
    }

    assert_eq!(
        guardian.observe(),
        ProcessIdentityObservation::Gone,
        "{GUARDIAN_SURVIVED_DETAIL}: {guardian:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exact_guardian_candidate_rejects_mismatch_alias_and_death() {
    let harness = Harness::start().await;
    let terminal_id = harness
        .create_cat(GUARDIAN_BINDING_SESSION_NAME)
        .await
        .manifest
        .terminal_id;
    let paths = harness.paths();
    let record: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let bound = harness
        .bind_cleanup_records(std::slice::from_ref(&record))
        .remove(0);
    let candidate = GuardianCandidate::read(&paths, terminal_id).unwrap();
    assert!(BoundCustody::bind(&record, candidate.clone()).is_ok());

    let mut stale_epoch = record.clone();
    stale_epoch.runtime_epoch = Uuid::new_v4();
    assert!(BoundCustody::bind(&stale_epoch, candidate.clone()).is_err());
    let mut stale_worker = record.clone();
    stale_worker.worker_id = Uuid::new_v4();
    assert!(BoundCustody::bind(&stale_worker, candidate.clone()).is_err());
    let mut stale_generation = record.clone();
    stale_generation.controller_generation += STALE_GENERATION_INCREMENT;
    assert!(BoundCustody::bind(&stale_generation, candidate.clone()).is_err());
    let mut stale_session = candidate.clone();
    stale_session.terminal_id = Uuid::new_v4();
    assert!(BoundCustody::bind(&record, stale_session).is_err());
    assert!(
        bind_candidates(
            &[bound.clone(), bound.clone()],
            vec![candidate.clone(), candidate.clone()],
        )
        .is_err()
    );

    let guardian = candidate.observation.guardian();
    assert!(guardian.signal(libc::SIGKILL).unwrap());
    wait_for_guardian_gone(guardian).await;
    discard_incomplete_coverage_profile(guardian.pid);
    assert!(bind_candidates(&[bound], vec![candidate]).is_err());
}

#[test]
fn missing_guardian_requires_explicit_fixture_mode() {
    let (_directory, paths, record) = fixture_record();
    let bound = bind_record(&paths, record);
    let records = std::slice::from_ref(&bound);

    assert!(
        capture(
            &paths,
            records,
            GuardianExpectation::RequiredOnSupportedPlatform
        )
        .is_err()
    );
    assert!(
        capture(
            &paths,
            records,
            GuardianExpectation::ExplicitlyAbsentFixture
        )
        .unwrap()
        .is_empty()
    );
}

async fn wait_for_guardian_gone(identity: RuntimeProcessIdentity) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while identity.observe() != ProcessIdentityObservation::Gone {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{GUARDIAN_GONE_TIMEOUT_DETAIL}: {identity:?}"
        );
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

fn fixture_record() -> (tempfile::TempDir, Paths, RuntimeRecord) {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let record = distinct_record(&paths, Uuid::new_v4());
    (directory, paths, record)
}

fn distinct_record(paths: &Paths, terminal_id: Uuid) -> RuntimeRecord {
    let identity = crate::support::ProcessIdentity {
        pid: TEST_WORKER_PID,
        start_time: TEST_WORKER_START_TIME,
    };
    let mut record = record_for_identity(paths, terminal_id, identity);
    record.child_pid = TEST_CHILD_PID;
    record.child_start_time = TEST_CHILD_START_TIME;
    record
}
