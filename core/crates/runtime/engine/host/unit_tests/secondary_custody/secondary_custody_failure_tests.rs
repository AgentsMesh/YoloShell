use super::*;
use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use crate::pty_custody::CustodyRuntimeIdentity;

#[path = "secondary_custody_disconnect_convergence_tests.rs"]
mod disconnect_convergence_tests;
#[path = "secondary_custody_exit_observation_race_tests.rs"]
mod exit_observation_race_tests;
#[path = "secondary_custody_guardian_monitor_tests.rs"]
mod guardian_monitor_tests;
#[path = "secondary_custody_liveness_adversarial_tests.rs"]
mod liveness_adversarial_tests;
#[path = "secondary_custody_release_ack_convergence_tests.rs"]
mod release_ack_convergence_tests;

const CURRENT_WORKER_FAIL_STOP_DETAIL: &str =
    "current Worker lost its established PTY custody control stream";
const CONTROL_EOF_DETAIL: &str = "read PTY custody control frame";
const RETIRED_LEASE_DETAIL: &str = "runtime has already retired from this Host generation";

#[tokio::test]
async fn same_live_worker_eof_fail_stops_without_settling_the_running_child() {
    let mut fixture = support::MacosHandshakeFixture::with_live_child(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);

    let convergence_started = tokio::time::Instant::now();
    disconnect.send(()).unwrap();
    server.await.unwrap();
    wait_until(&fixture.host, || {
        fixture.host.request_drain.fail_stop_detail().is_some()
    })
    .await;

    let detail = fixture.host.request_drain.fail_stop_detail().unwrap();
    assert!(
        convergence_started.elapsed()
            >= crate::session_timing::PROCESS_CAPABILITY_EXIT_EVENT_CONVERGENCE_TIMEOUT
    );
    assert!(detail.contains(CURRENT_WORKER_FAIL_STOP_DETAIL));
    assert!(detail.contains(CONTROL_EOF_DETAIL));
    assert_eq!(lease.state(), SecondaryCustodyState::Disconnected);
    assert!(lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert_eq!(
        ProcessIdentity {
            pid: record.child_pid,
            start_time: record.child_start_time,
        }
        .observe(),
        ProcessIdentityObservation::Matching
    );
    assert!(Arc::ptr_eq(&lease, &current(&fixture.host, &record)));
}

#[tokio::test]
async fn proven_gone_worker_eof_actively_settles_exact_platform_custody() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let gone_worker = reaped_process_identity();
    lease.probe_worker_identity(gone_worker);
    fixture.close_worker_aliases();

    disconnect.send(()).unwrap();
    server.await.unwrap();
    wait_until(&fixture.host, || {
        fixture
            .host
            .secondary_custody
            .current(&CustodyRuntimeIdentity::from_record(&record))
            .is_none()
    })
    .await;

    assert_eq!(gone_worker.observe(), ProcessIdentityObservation::Gone);
    assert_eq!(lease.state(), SecondaryCustodyState::Released);
    assert!(!lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    let retry = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();
    assert!(retry.to_string().contains(RETIRED_LEASE_DETAIL));
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
}

#[tokio::test]
async fn proven_gone_worker_settles_while_control_stream_remains_open() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let gone_worker = reaped_process_identity();
    lease.probe_worker_identity(gone_worker);
    fixture.close_worker_aliases();

    wait_until(&fixture.host, || {
        fixture
            .host
            .secondary_custody
            .current(&CustodyRuntimeIdentity::from_record(&record))
            .is_none()
    })
    .await;

    assert_eq!(lease.state(), SecondaryCustodyState::Released);
    assert!(!lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    release.send(()).unwrap();
    server.await.unwrap();
}

fn reaped_process_identity() -> ProcessIdentity {
    let identity = crate::host::reaped_host_test_process_identity();
    assert_eq!(identity.observe(), ProcessIdentityObservation::Gone);
    identity
}

fn current(host: &Host, record: &RuntimeRecord) -> Arc<SecondaryCustodyLease> {
    host.secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(record))
        .unwrap()
}

async fn wait_until(host: &Host, ready: impl Fn() -> bool) {
    tokio::time::timeout(host.worker_ready_timeout, async {
        while !ready() {
            tokio::time::sleep(host.worker_ready_poll_interval).await;
        }
    })
    .await
    .unwrap();
}
