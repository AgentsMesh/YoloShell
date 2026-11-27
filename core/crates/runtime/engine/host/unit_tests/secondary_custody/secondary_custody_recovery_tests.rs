use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;
use std::sync::atomic::Ordering;

const UNEXPECTED_OPERATION_COMPLETION: &str = "custody operation completed unexpectedly";

#[tokio::test]
async fn post_ready_predecessor_fault_requires_a_higher_host_generation() {
    let mut predecessor_fixture = support::MacosHandshakeFixture::new(None);
    let predecessor_record = predecessor_fixture.record.clone();
    let terminal_id = predecessor_record.terminal_id;
    let (predecessor_server, disconnect_predecessor) = predecessor_fixture.serve_holding();
    predecessor_fixture
        .host
        .ensure_secondary_custody(&predecessor_record)
        .await
        .unwrap();
    let predecessor = current(&predecessor_fixture.host, &predecessor_record);

    let mut successor_fixture = support::MacosHandshakeFixture::new(Some(terminal_id));
    let successor_record = successor_fixture.record.clone();
    let (successor_server, disconnect_successor) = successor_fixture.serve_holding();
    successor_fixture
        .host
        .ensure_secondary_custody(&successor_record)
        .await
        .unwrap();
    let successor = current(&successor_fixture.host, &successor_record);
    assert_eq!(
        predecessor_fixture.host.controller_generation,
        successor_fixture.host.controller_generation
    );
    successor.available.store(false, Ordering::Release);
    successor.set_state(SecondaryCustodyState::Activating);
    predecessor_fixture
        .host
        .secondary_custody
        .install(Arc::clone(&successor))
        .unwrap();

    disconnect_predecessor.send(()).unwrap();
    predecessor_server.await.unwrap();
    wait_until(&predecessor_fixture.host, || {
        predecessor_fixture
            .host
            .request_drain
            .fail_stop_detail()
            .is_some()
    })
    .await;
    assert!(!predecessor.settlement_is_active());
    assert!(predecessor.retains_platform());

    let error = predecessor_fixture
        .host
        .custodied_worker_stream(&successor_record)
        .await
        .unwrap_err();
    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert_eq!(successor.state(), SecondaryCustodyState::Activating);
    assert!(successor.is_wire_ready() && !successor.is_ready());
    assert!(successor.retains_platform());
    assert!(Arc::ptr_eq(
        &successor,
        &current(&predecessor_fixture.host, &successor_record)
    ));

    disconnect_successor.send(()).unwrap();
    successor_server.await.unwrap();
}

#[tokio::test]
async fn cancellation_after_activation_before_availability_synchronously_fail_stops() {
    let mut predecessor_fixture = support::MacosHandshakeFixture::new(None);
    let predecessor_record = predecessor_fixture.record.clone();
    let terminal_id = predecessor_record.terminal_id;
    let (predecessor_server, disconnect_predecessor) = predecessor_fixture.serve_holding();
    predecessor_fixture
        .host
        .ensure_secondary_custody(&predecessor_record)
        .await
        .unwrap();

    let mut successor_fixture = support::MacosHandshakeFixture::new(Some(terminal_id));
    successor_fixture.publish_for_host(&predecessor_fixture.host);
    let successor_record = successor_fixture.record.clone();
    let (successor_server, disconnect_successor) = successor_fixture.serve_holding();
    {
        let operation = predecessor_fixture
            .host
            .ensure_secondary_custody(&successor_record);
        tokio::pin!(operation);
        tokio::select! {
            result = &mut operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
            () = wait_until(&predecessor_fixture.host, || {
                predecessor_fixture.host.secondary_custody
                    .current(&CustodyRuntimeIdentity::from_record(&successor_record))
                    .is_some_and(|lease| lease.state() == SecondaryCustodyState::Activating)
            }) => {}
        }
    }

    let successor = current(&predecessor_fixture.host, &successor_record);
    assert!(
        predecessor_fixture
            .host
            .request_drain
            .fail_stop_detail()
            .is_some()
    );
    assert_eq!(successor.state(), SecondaryCustodyState::Activating);
    assert!(successor.is_wire_ready() && !successor.is_ready());
    assert!(successor.retains_platform());

    disconnect_predecessor.send(()).unwrap();
    predecessor_server.await.unwrap();
    disconnect_successor.send(()).unwrap();
    successor_server.await.unwrap();
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
