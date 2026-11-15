use super::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

const TEST_SUCCESSOR_GENERATION: u64 = 2;
const SINGLE_EXPECTATION_COUNT: usize = 1;
const MULTIPLE_EXPECTATION_COUNT: usize = 2;
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(1);
const TEST_TIMEOUT: Duration = Duration::from_millis(20);
const TEST_FAILURE: &str = "injected adoption failure";
const TEST_TRANSIENT_GET_SESSION_FAILURE: &str = "injected transient get-session failure";
const ADOPTION_TEST_SESSION_NAMES: &[&str] = &["adoption-barrier-first", "adoption-barrier-second"];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replacement_without_runtimes_has_an_empty_adoption_barrier() {
    let mut harness = Harness::start().await;
    let predecessor = harness.host_identity();

    harness.restart_host_hard().await;

    assert_ne!(harness.host_identity(), predecessor);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replacement_returns_after_all_surviving_runtimes_are_adopted() {
    let mut harness = Harness::start().await;
    let mut trusted = Vec::with_capacity(ADOPTION_TEST_SESSION_NAMES.len());
    for name in ADOPTION_TEST_SESSION_NAMES {
        let terminal_id = harness.create_cat(name).await.manifest.terminal_id;
        trusted
            .push(read_json::<RuntimeRecord>(&harness.paths().runtime_record(terminal_id)).unwrap());
    }

    harness.restart_host_hard().await;

    let generation = harness
        .client()
        .status()
        .await
        .unwrap()
        .controller_generation;
    for expected in trusted {
        let record: RuntimeRecord =
            read_json(&harness.paths().runtime_record(expected.terminal_id)).unwrap();
        assert!(record.has_same_process_identity(&expected));
        assert_eq!(record.controller_generation, generation);
        let view = harness
            .client()
            .get_session(expected.terminal_id)
            .await
            .unwrap();
        let runtime = view.runtime.unwrap();
        assert_eq!(runtime.runtime_epoch, expected.runtime_epoch);
        assert_eq!(runtime.worker_id, expected.worker_id);
        assert_eq!(runtime.status, TerminalRuntimeStatus::Running);
    }
}

#[tokio::test]
async fn empty_adoption_set_completes_without_probing() {
    let probes = AtomicUsize::new(usize::default());
    wait_for_adoption_progress(
        usize::default(),
        Duration::ZERO,
        Duration::ZERO,
        || Ok(()),
        || {
            probes.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(AdoptionProgress::Complete))
        },
    )
    .await
    .unwrap();
    assert_eq!(probes.load(Ordering::SeqCst), usize::default());
}

#[tokio::test]
async fn adoption_waits_until_every_expected_runtime_is_complete() {
    let mut progress = VecDeque::from([
        AdoptionProgress::Pending(ADOPTION_PENDING_DETAIL.into()),
        AdoptionProgress::Complete,
    ]);
    wait_for_adoption_progress(
        MULTIPLE_EXPECTATION_COUNT,
        TEST_TIMEOUT,
        TEST_POLL_INTERVAL,
        || Ok(()),
        || std::future::ready(Ok(progress.pop_front().unwrap())),
    )
    .await
    .unwrap();
    assert!(progress.is_empty());
}

#[tokio::test]
async fn transient_get_session_failure_is_retried_with_session_context() {
    let terminal_id = Uuid::new_v4();
    let pending = pending_session_view_error(terminal_id, TEST_TRANSIENT_GET_SESSION_FAILURE);
    let AdoptionProgress::Pending(detail) = &pending else {
        unreachable!();
    };
    assert!(detail.contains(&terminal_id.to_string()));
    assert!(detail.contains(TEST_TRANSIENT_GET_SESSION_FAILURE));
    let mut progress = VecDeque::from([pending, AdoptionProgress::Complete]);

    wait_for_adoption_progress(
        SINGLE_EXPECTATION_COUNT,
        TEST_TIMEOUT,
        TEST_POLL_INTERVAL,
        || Ok(()),
        || std::future::ready(Ok(progress.pop_front().unwrap())),
    )
    .await
    .unwrap();
    assert!(progress.is_empty());
}

#[tokio::test]
async fn adoption_failure_is_not_retried() {
    let probes = AtomicUsize::new(usize::default());
    let error = wait_for_adoption_progress(
        SINGLE_EXPECTATION_COUNT,
        TEST_TIMEOUT,
        TEST_POLL_INTERVAL,
        || Ok(()),
        || {
            probes.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Err(TEST_FAILURE.into()))
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, TEST_FAILURE);
    assert_eq!(probes.load(Ordering::SeqCst), SINGLE_EXPECTATION_COUNT);
}

#[tokio::test]
async fn adoption_timeout_reports_the_pending_runtime() {
    let error = wait_for_adoption_progress(
        SINGLE_EXPECTATION_COUNT,
        Duration::ZERO,
        Duration::ZERO,
        || Ok(()),
        || std::future::ready(Ok(AdoptionProgress::Pending(TEST_FAILURE.into()))),
    )
    .await
    .unwrap_err();
    assert!(error.contains(ADOPTION_TIMEOUT_DETAIL));
}

#[test]
fn adoption_accepts_only_the_exact_runtime_at_the_successor_generation() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut record = super::super::runtime_record_test_support::record_for_identity(
        &paths,
        Uuid::new_v4(),
        identity,
    );
    let bound = super::super::runtime_record_test_support::bind_record(&paths, record.clone());
    let expected = RuntimeAdoptionExpectation::new(bound);
    record.controller_generation = TEST_SUCCESSOR_GENERATION;
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let view = super::super::runtime_record_test_support::session_view(&record);

    assert_eq!(
        evaluate_runtime(&expected, &record, Some(&view), TEST_SUCCESSOR_GENERATION).unwrap(),
        AdoptionProgress::Complete
    );

    record.worker_id = Uuid::new_v4();
    assert!(evaluate_runtime(&expected, &record, Some(&view), TEST_SUCCESSOR_GENERATION).is_err());
}
