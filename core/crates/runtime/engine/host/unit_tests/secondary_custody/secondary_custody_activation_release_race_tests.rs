use super::*;
use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use crate::pty_custody::CustodyRuntimeIdentity;

const UNEXPECTED_OPERATION_COMPLETION: &str = "custody operation completed unexpectedly";
const RELEASED_DURING_ACTIVATION_DETAIL: &str =
    "released PTY custody before Host availability publication";
const LIVE_WORKER_FIXTURE_COMMAND: &str = "/bin/sleep";
const LIVE_WORKER_FIXTURE_DURATION: &str = "30";

#[tokio::test]
async fn exact_release_during_predecessor_supersede_never_publishes_availability() {
    let mut predecessor_fixture = support::MacosHandshakeFixture::new(None);
    let predecessor_record = predecessor_fixture.record.clone();
    let terminal_id = predecessor_record.terminal_id;
    let (predecessor_server, supersede_received, finish_supersede) =
        predecessor_fixture.serve_held_supersede();
    predecessor_fixture
        .host
        .ensure_secondary_custody(&predecessor_record)
        .await
        .unwrap();
    let predecessor_aliases = predecessor_fixture.take_worker_aliases();

    let mut successor_fixture = support::MacosHandshakeFixture::new(Some(terminal_id));
    successor_fixture.publish_for_host(&predecessor_fixture.host);
    let successor_record = successor_fixture.record.clone();
    let (successor_server, release) = successor_fixture.serve_release(
        support::ReleaseIdentity::Exact,
        ReleaseResponse::ReadAcknowledgement,
    );
    let operation = predecessor_fixture
        .host
        .ensure_secondary_custody(&successor_record);
    tokio::pin!(operation);
    tokio::select! {
        reached = supersede_received => reached.unwrap(),
        result = &mut operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
    }
    let successor = current(&predecessor_fixture.host, &successor_record);
    assert_eq!(successor.state(), SecondaryCustodyState::Activating);

    successor_fixture.close_worker_aliases();
    release.send(()).unwrap();
    successor_server.await.unwrap();
    wait_until(&predecessor_fixture.host, || {
        successor.state() == SecondaryCustodyState::Released
    })
    .await;
    assert!(!successor.is_ready());
    assert!(!successor.retains_platform());

    predecessor_aliases.close();
    finish_supersede.send(()).unwrap();
    predecessor_server.await.unwrap();
    let error = operation.await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains(RELEASED_DURING_ACTIVATION_DETAIL)
    );
    assert!(
        predecessor_fixture
            .host
            .request_drain
            .fail_stop_detail()
            .is_none()
    );
}

#[tokio::test]
async fn proven_gone_predecessor_eof_during_supersede_converges_successor() {
    let mut predecessor_fixture = support::MacosHandshakeFixture::new(None);
    let predecessor_record = predecessor_fixture.record.clone();
    let terminal_id = predecessor_record.terminal_id;
    let (predecessor_server, supersede_received, close_predecessor) =
        predecessor_fixture.serve_held_supersede_eof();
    predecessor_fixture
        .host
        .ensure_secondary_custody(&predecessor_record)
        .await
        .unwrap();
    let predecessor = current(&predecessor_fixture.host, &predecessor_record);
    let predecessor_aliases = predecessor_fixture.take_worker_aliases();

    let mut successor_fixture = support::MacosHandshakeFixture::new(Some(terminal_id));
    successor_fixture.publish_for_host(&predecessor_fixture.host);
    let successor_record = successor_fixture.record.clone();
    let (successor_server, disconnect_successor) = successor_fixture.serve_holding();
    let operation = predecessor_fixture
        .host
        .ensure_secondary_custody(&successor_record);
    tokio::pin!(operation);
    tokio::select! {
        reached = supersede_received => reached.unwrap(),
        result = &mut operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
    }

    let mut worker = crate::process::spawn_waitable_test_child(
        std::process::Command::new(LIVE_WORKER_FIXTURE_COMMAND).arg(LIVE_WORKER_FIXTURE_DURATION),
    );
    let worker_identity = ProcessIdentity::capture(worker.id()).unwrap();
    predecessor.probe_worker_identity(worker_identity);
    let mut states = predecessor.subscribe_state();
    close_predecessor.send(()).unwrap();
    predecessor_server.await.unwrap();
    wait_for_state(&mut states, SecondaryCustodyState::Disconnected).await;
    assert!(
        predecessor_fixture
            .host
            .request_drain
            .fail_stop_detail()
            .is_none()
    );
    worker.kill().unwrap();
    worker.wait().unwrap();
    predecessor_aliases.close();
    operation.await.unwrap();

    assert_eq!(worker_identity.observe(), ProcessIdentityObservation::Gone);
    assert_eq!(predecessor.state(), SecondaryCustodyState::Superseded);
    let successor = current(&predecessor_fixture.host, &successor_record);
    assert!(successor.is_ready());
    assert!(
        predecessor_fixture
            .host
            .request_drain
            .fail_stop_detail()
            .is_none()
    );
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

async fn wait_for_state(
    states: &mut tokio::sync::watch::Receiver<SecondaryCustodyState>,
    expected: SecondaryCustodyState,
) {
    while *states.borrow_and_update() != expected {
        states.changed().await.unwrap();
    }
}
