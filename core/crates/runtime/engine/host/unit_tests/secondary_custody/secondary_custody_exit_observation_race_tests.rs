use super::*;
use crate::process::{ProcessIdentity, ProcessIdentityObservation, ProcessIdentityRelation};

#[tokio::test]
async fn guardian_first_waits_for_worker_exit_observation() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let mut states = lease.subscribe_state();
    let _guardian =
        guardian_monitor_tests::install_guardian_relation(&fixture, ProcessIdentityRelation::Gone);

    wait_for_state(&fixture.host, &mut states, SecondaryCustodyState::Failed).await;
    assert!(worker_identity(&record).matches());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    assert!(!lease.settlement_is_active());

    let gone_worker = reaped_process_identity();
    lease.probe_worker_identity(gone_worker);
    fixture.close_worker_aliases();
    wait_for_state(&fixture.host, &mut states, SecondaryCustodyState::Released).await;

    assert_eq!(gone_worker.observe(), ProcessIdentityObservation::Gone);
    assert!(!lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    release.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn worker_first_settles_after_guardian_exit_without_fail_stop() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let mut states = lease.subscribe_state();
    let gone_worker = reaped_process_identity();
    lease.probe_worker_identity(gone_worker);
    let mut fault = super::super::super::failure::test_hook::install(record.terminal_id);

    disconnect.send(()).unwrap();
    server.await.unwrap();
    fault.reached(&fixture.host).await;
    assert_eq!(lease.state(), SecondaryCustodyState::Disconnected);
    let _guardian =
        guardian_monitor_tests::install_guardian_relation(&fixture, ProcessIdentityRelation::Gone);
    assert_eq!(gone_worker.observe(), ProcessIdentityObservation::Gone);
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());

    fixture.close_worker_aliases();
    fault.resume();
    wait_for_state(&fixture.host, &mut states, SecondaryCustodyState::Released).await;

    assert!(!lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
}

fn worker_identity(record: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    }
}

async fn wait_for_state(
    host: &Host,
    states: &mut tokio::sync::watch::Receiver<SecondaryCustodyState>,
    expected: SecondaryCustodyState,
) {
    tokio::time::timeout(host.worker_ready_timeout, async {
        while *states.borrow_and_update() != expected {
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
}
