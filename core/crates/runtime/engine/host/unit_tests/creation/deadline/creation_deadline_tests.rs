use crate::host::tests::*;
use super::*;

pub(super) const CREATE_DEADLINE_TEST_TIMEOUT: Duration = Duration::from_millis(40);
const CREATE_DEADLINE_ELAPSED_BEFORE_WAIT: Duration = Duration::from_millis(30);
const CREATE_DEADLINE_TEST_SESSION_NAME: &str = "expired-before-prepare";

#[tokio::test(start_paused = true)]
async fn creation_deadline_expiry_cannot_prepare_a_launch() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, binding, ledger_path) =
        reserve_test_creation(&host, directory.path(), CREATE_DEADLINE_TEST_SESSION_NAME);
    let deadline = HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT);
    tokio::time::advance(CREATE_DEADLINE_TEST_TIMEOUT).await;

    let error = host
        .prepare_creation_launch_attempt_before(&ledger_path, binding.result, deadline)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_transition_ledger(&ledger_path, &[binding]);
}

#[tokio::test(start_paused = true)]
async fn creation_deadline_starts_before_waiting_for_the_session_lock() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.worker_ready_timeout = CREATE_DEADLINE_TEST_TIMEOUT;
    let (request, binding, ledger_path) =
        reserve_test_creation(&host, directory.path(), CREATE_DEADLINE_TEST_SESSION_NAME);
    let guard = host.session_locks.lock(binding.result.terminal_id).await;
    let create = host.create_session(request);
    let release = async move {
        tokio::task::yield_now().await;
        tokio::time::advance(CREATE_DEADLINE_TEST_TIMEOUT).await;
        drop(guard);
    };

    let (result, ()) = tokio::join!(create, release);

    assert_eq!(
        result.unwrap_err().code,
        crate::protocol::error_code::DELIVERY_UNKNOWN
    );
    assert!(!host.paths.session_dir(binding.result.terminal_id).exists());
    assert_transition_ledger(&ledger_path, &[binding]);
}

#[tokio::test(start_paused = true)]
async fn creation_deadline_expiry_retires_launch_before_the_spawn_boundary() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let launch = shell_resume_launch(&host, &manifest);
    host.publish_worker_launch(&launch).unwrap();
    let deadline = HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT);
    tokio::time::advance(CREATE_DEADLINE_TEST_TIMEOUT).await;

    let error = session_spawn_wave4_error(
        host.spawn_published_worker_before(
            &manifest,
            &launch,
            UnstartedLaunchPolicy::Retire,
            deadline,
        )
        .await,
    );

    assert!(error.to_string().contains(WORKER_START_DEADLINE_EXPIRED));
    assert!(
        host.namespace
            .file_is_missing(&host.paths.worker_launch(manifest.terminal_id))
            .unwrap()
    );
}

#[tokio::test(start_paused = true)]
async fn creation_deadline_expiry_cannot_publish_a_prepared_launch() {
    let fixture = CreationCheckpointFixture::new(false).await;
    let launch = fixture
        .host
        .prepare_creation_worker_launch(&fixture.binding, &fixture.planned)
        .unwrap();
    let deadline = HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT);
    tokio::time::advance(CREATE_DEADLINE_TEST_TIMEOUT).await;

    let error = fixture
        .host
        .publish_creation_worker_launch_before(&launch, deadline)
        .unwrap_err();

    assert!(error.is::<CreationPublicationDeadlineExpired>());
    assert!(
        fixture
            .host
            .namespace
            .file_is_missing(&fixture.host.paths.worker_launch(launch.terminal_id))
            .unwrap()
    );
}

#[tokio::test(start_paused = true)]
async fn creation_deadline_is_not_reset_before_worker_wait() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.worker_ready_poll_interval = CREATE_DEADLINE_TEST_TIMEOUT;
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let launch = shell_resume_launch(&host, &manifest);
    let started_at = tokio::time::Instant::now();
    let deadline = HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT);
    tokio::time::advance(CREATE_DEADLINE_ELAPSED_BEFORE_WAIT).await;

    let result = host
        .await_worker_start_before(&manifest, launch.runtime_epoch, launch.worker_id, deadline)
        .await
        .unwrap();

    assert_eq!(
        tokio::time::Instant::now().duration_since(started_at),
        CREATE_DEADLINE_TEST_TIMEOUT
    );
    assert_eq!(uncertain_start_detail(result), MISSING_WORKER_DETAIL);
}

#[tokio::test(start_paused = true)]
async fn creation_deadline_bounds_worker_authentication() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.worker_ready_poll_interval = CREATE_DEADLINE_TEST_TIMEOUT;
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let launch = shell_resume_launch(&host, &manifest);
    let record = runtime_for_creation(&host, &launch, BootstrapDelivery::NotRequired, false);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let socket_cleanup = SocketCleanup::new(&record.socket_path).unwrap();
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &record).unwrap();
    let started_at = tokio::time::Instant::now();
    let deadline = HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT);
    tokio::time::advance(CREATE_DEADLINE_ELAPSED_BEFORE_WAIT).await;

    let result = host
        .await_worker_start_before(&manifest, launch.runtime_epoch, launch.worker_id, deadline)
        .await
        .unwrap();

    assert_eq!(
        tokio::time::Instant::now().duration_since(started_at),
        CREATE_DEADLINE_TEST_TIMEOUT
    );
    assert_eq!(uncertain_start_detail(result), MISSING_WORKER_DETAIL);
    drop(listener);
    drop(socket_cleanup);
    assert!(!record.socket_path.exists());
}

include!("creation_deadline_adversarial_tests.rs");
include!("../runtime/creation_readiness_budget_tests.rs");
include!("creation_deadline_publication_tests.rs");
include!("../runtime/creation_public_diagnostics_tests.rs");
