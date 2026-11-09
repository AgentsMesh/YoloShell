use super::*;
use crate::model::{RuntimeView, TerminalRuntimeStatus};
use crate::paths::atomic_write_json;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, read_frame, write_frame};
use crate::pty_custody::{
    CustodyHostAck, CustodyWorkerActivated, CustodyWorkerCommit, CustodyWorkerReadyDisposition,
};
use std::time::Duration;

const STATUS_RPC_TIMEOUT: Duration = Duration::from_millis(50);
const ADOPTION_HOLD_TIMEOUT: Duration = Duration::from_millis(100);
const STATUS_OUTPUT_SEQUENCE: u64 = 0;
const EARLY_STATUS_COMPLETION: &str = "status completed during custody adoption";
const PREPARED_GUARDIAN_REQUIRED_DETAIL: &str = "prepared guardian binding is required";

#[tokio::test]
async fn status_timeout_never_cancels_custody_adoption() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    fixture.host.worker_status_timeout = STATUS_RPC_TIMEOUT;
    let record = fixture.record.clone();
    let (listener, export, _, custody_path, server_record) = fixture.take_parts();
    let (prepared, reached_prepared) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut custody, offer, request) =
            handshake_support::through_prepared(&listener, export, &server_record).await;
        prepared.send(()).unwrap();
        released.await.unwrap();
        let guarded: crate::pty_custody::CustodyDurableRecord =
            crate::paths::read_json(&custody_path).unwrap();
        let Some(binding) = guarded
            .prepared_guardian_candidate()
            .and_then(|candidate| candidate.guardian)
        else {
            panic!("{PREPARED_GUARDIAN_REQUIRED_DETAIL}");
        };
        let armed = guarded
            .arm_prepared_guardian(&request, &offer, binding)
            .unwrap();
        atomic_write_json(&custody_path, &armed).unwrap();
        custody
            .write(&CustodyWorkerCommit::new(&offer))
            .await
            .unwrap();
        let ready: CustodyHostAck = custody.read().await.unwrap();
        assert!(ready.matches_ready(&offer, request.controller_generation));
        let activation =
            CustodyWorkerActivated::new(&offer, request.controller_generation).unwrap();
        custody
            .write(&CustodyWorkerReadyDisposition::activated(activation))
            .await
            .unwrap();
        serve_running_status(&listener, &server_record).await;
        let _custody = custody;
        std::future::pending::<()>().await;
    });

    let mut status = Box::pin(fixture.host.worker_status(&record));
    tokio::select! {
        reached = reached_prepared => reached.unwrap(),
        result = &mut status => panic!("{EARLY_STATUS_COMPLETION}: {result:?}"),
    }
    assert!(
        tokio::time::timeout(ADOPTION_HOLD_TIMEOUT, &mut status)
            .await
            .is_err()
    );
    assert!(!fixture.host.rpc_custody_ready(&record));
    assert!(!fixture.host.request_drain.is_draining());

    release.send(()).unwrap();
    let view = tokio::time::timeout(fixture.host.worker_ready_timeout, &mut status)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(view.status, TerminalRuntimeStatus::Running);
    assert!(fixture.host.rpc_custody_ready(&record));
    assert!(!fixture.host.request_drain.is_draining());
    server.abort();
    let _ = server.await;
}

async fn serve_running_status(listener: &UnixListener, record: &RuntimeRecord) {
    let stream = handshake_support::authenticate(listener, record).await;
    let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
    let request = stream.request().await.unwrap();
    let response = ResponseEnvelope::success(&request, &running_view(record)).unwrap();
    stream.respond(&response).await.unwrap();
}

fn running_view(record: &RuntimeRecord) -> RuntimeView {
    RuntimeView {
        presentation: Default::default(),
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        worker_pid: record.worker_pid,
        worker_start_time: record.worker_start_time,
        child_pid: record.child_pid,
        child_start_time: record.child_start_time,
        status: TerminalRuntimeStatus::Running,
        last_output_sequence: STATUS_OUTPUT_SEQUENCE,
        exit: None,
        bootstrap_delivery: record.bootstrap_delivery,
        capabilities: Some(Vec::new()),
        fault_reap: None,
    }
}
