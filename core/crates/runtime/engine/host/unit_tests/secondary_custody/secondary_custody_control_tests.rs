use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

const FORGED_RELEASE_DETAIL: &str = "Release does not match its exact offer";
const RELEASE_TIMEOUT_DETAIL: &str = "custody control exchange timed out";
const MONITOR_PANIC_DETAIL: &str = "injected PTY custody monitor panic";
const RETIRED_LEASE_DETAIL: &str = "runtime has already retired from this Host generation";

#[tokio::test]
async fn exact_worker_release_drops_platform_before_ack_and_removes_exact_entry() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_release(
        support::ReleaseIdentity::Exact,
        ReleaseResponse::ReadAcknowledgement,
    );
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    assert!(lease.is_ready() && lease.retains_platform());

    fixture.close_worker_aliases();
    release.send(()).unwrap();
    server.await.unwrap();
    wait_until(&fixture.host, || {
        lease.state() == SecondaryCustodyState::Released
    })
    .await;

    assert!(!lease.is_ready() && !lease.retains_platform());
    assert!(
        fixture
            .host
            .secondary_custody
            .current(&CustodyRuntimeIdentity::from_record(&record))
            .is_none()
    );
    let retry = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();
    assert!(retry.to_string().contains(RETIRED_LEASE_DETAIL));
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
}

#[tokio::test]
async fn forged_release_from_current_worker_fail_stops_without_active_settlement() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_release(
        support::ReleaseIdentity::Forged,
        ReleaseResponse::DropWithoutReading,
    );
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);

    release.send(()).unwrap();
    server.await.unwrap();
    wait_until(&fixture.host, || {
        fixture.host.request_drain.fail_stop_detail().is_some()
    })
    .await;

    assert_eq!(lease.state(), SecondaryCustodyState::Failed);
    assert!(!lease.is_ready() && lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(FORGED_RELEASE_DETAIL)
    );
    assert!(Arc::ptr_eq(&lease, &current(&fixture.host, &record)));
}

#[tokio::test]
async fn release_without_settlement_times_out_and_fail_stops_host() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_release(
        support::ReleaseIdentity::Exact,
        ReleaseResponse::DropWithoutReading,
    );
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);

    release.send(()).unwrap();
    server.await.unwrap();
    wait_until(&fixture.host, || {
        fixture.host.request_drain.fail_stop_detail().is_some()
    })
    .await;

    assert_eq!(lease.state(), SecondaryCustodyState::Failed);
    assert!(!lease.is_ready() && lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(RELEASE_TIMEOUT_DETAIL)
    );
    assert!(Arc::ptr_eq(&lease, &current(&fixture.host, &record)));
}

#[tokio::test]
async fn current_worker_monitor_panic_is_contained_by_host_fail_stop() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);

    lease.panic_monitor().await;
    wait_until(&fixture.host, || {
        fixture.host.request_drain.fail_stop_detail().is_some()
    })
    .await;

    assert_eq!(lease.state(), SecondaryCustodyState::Failed);
    assert!(!lease.is_ready() && lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(MONITOR_PANIC_DETAIL)
    );
    release.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn registry_drop_cancels_monitor_ownership_without_an_arc_cycle() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let weak = Arc::downgrade(&lease);

    drop(lease);
    drop(fixture);
    wait_for_drop(&weak).await;
    release.send(()).unwrap();
    server.await.unwrap();
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

async fn wait_for_drop(lease: &Weak<SecondaryCustodyLease>) {
    tokio::time::timeout(crate::session_timing::WORKER_READY_TIMEOUT, async {
        while lease.strong_count() != usize::default() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
