use super::super::constants::{LOOPBACK_HOST, TRANSPORT_EVENT_TIMEOUT};
use super::super::factory::LiveSshFactory;
use super::super::sshd::SshdFixture;
use super::guard::read_status_until;
use super::open_initial_with_probe_barrier;
use super::provisional::ProbeBarrier;
use std::sync::Arc;
use yolo_process_test_support::{ProcessIdentity, ProcessIdentityObservation};
use yolo_remote::{RemoteEndpoint, RemoteRuntimeSpec};
use yolo_runtime::paths::Paths;

const OPEN_TASK_COMPLETED_DETAIL: &str =
    "SSH runtime open completed before its provisional guard was released";
const OPEN_TASK_NOT_CANCELLED_DETAIL: &str = "SSH runtime open task was not cancelled";
const DETACHED_HOST_SURVIVED_DETAIL: &str =
    "detached Host survived cancellation before guard publication";
const PROVISIONAL_GUARD_NOT_REACHED_DETAIL: &str =
    "provisional detached runtime guard did not reach its test barrier";
const CANCELLED_OPEN_SETTLEMENT_EXCEEDED_DETAIL: &str =
    "cancelled SSH runtime open did not settle within its operation budget";
const CANCELLATION_SETTLEMENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(4);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancellation_after_runtime_exec_cannot_leak_the_unpublished_detached_host() {
    let sshd = Arc::new(SshdFixture::start().await);
    let endpoint = RemoteEndpoint::new(sshd.user(), LOOPBACK_HOST, sshd.address().port()).unwrap();
    let factory = LiveSshFactory::new(
        endpoint,
        sshd.known_hosts(sshd.address().port()),
        sshd.identity(),
    );
    let runtime = RemoteRuntimeSpec::new(
        crate::runtime_binary().canonicalize().unwrap(),
        sshd.remote_home(),
    )
    .unwrap();
    let barrier = ProbeBarrier::default();
    let task_sshd = Arc::clone(&sshd);
    let task_factory = Arc::clone(&factory);
    let task_barrier = barrier.clone();
    let opening = tokio::spawn(async move {
        open_initial_with_probe_barrier(&task_sshd, &task_factory, &runtime, task_barrier).await
    });
    tokio::time::timeout(TRANSPORT_EVENT_TIMEOUT, barrier.wait_until_reached())
        .await
        .expect(PROVISIONAL_GUARD_NOT_REACHED_DETAIL);
    let socket = Paths::new(sshd.remote_home()).host_socket().unwrap();
    let client = yolo_client::Client::managed_local(sshd.remote_home(), socket).unwrap();
    let status = read_status_until(
        &client,
        tokio::time::Instant::now() + TRANSPORT_EVENT_TIMEOUT,
    )
    .await
    .unwrap();
    let host = ProcessIdentity::capture(status.host_pid).unwrap();
    assert!(!opening.is_finished(), "{OPEN_TASK_COMPLETED_DETAIL}");

    opening.abort();
    match tokio::time::timeout(CANCELLATION_SETTLEMENT_TIMEOUT, opening)
        .await
        .expect(CANCELLED_OPEN_SETTLEMENT_EXCEEDED_DETAIL)
    {
        Err(error) => assert!(
            error.is_cancelled(),
            "{OPEN_TASK_NOT_CANCELLED_DETAIL}: {error}"
        ),
        Ok(_) => panic!("{OPEN_TASK_NOT_CANCELLED_DETAIL}"),
    }
    barrier.release();
    let deadline = tokio::time::Instant::now() + CANCELLATION_SETTLEMENT_TIMEOUT;
    while host.observe() != ProcessIdentityObservation::Gone
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(super::super::constants::DETACHED_HOST_PROBE_POLL_INTERVAL).await;
    }
    assert_eq!(
        host.observe(),
        ProcessIdentityObservation::Gone,
        "{DETACHED_HOST_SURVIVED_DETAIL}: {host:?}"
    );
}
