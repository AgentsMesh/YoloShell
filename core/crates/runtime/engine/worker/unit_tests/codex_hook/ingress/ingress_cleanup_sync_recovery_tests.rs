use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_live_source::PeerEvidence;
use super::ingress_test_support::{IngressEndpoint, PendingIngressExchange};
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture, SECOND_EVENT_SEQUENCE};
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{CodexHookIntake, CodexOutbox};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

const CLEANUP_SYNC_HOME: &str = "ingress-cleanup-sync-recovery-home";
const CLEANUP_OVERLAP_HOME: &str = "ingress-cleanup-overlap-recovery-home";
const SINGLE_RECEIPT: usize = 1;
const OVERLAP_TIMEOUT: Duration = Duration::from_millis(100);
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(3);

type Server = super::super::server::CodexHookServer;
type Runtime = super::super::runtime::CodexHookRuntime;
type Handler =
    tokio::task::JoinHandle<anyhow::Result<super::super::outcome::CodexHookHandleOutcome>>;

#[tokio::test]
async fn cleanup_sync_failure_defers_without_losing_live_authority() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(CLEANUP_SYNC_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let occurrence = fixture.start(FIRST_EVENT_SEQUENCE);
    let outbox = endpoint.outbox(&occurrence);
    let intake = intake(&endpoint, &occurrence);
    let gate = runtime.gate_next_response_for_test();
    let (handler, pending) = begin_event(&server, &endpoint, &occurrence).await;
    gate.wait_until_entered().await;

    let (accepted, peer) = pending.observe_accepted_pending_cleanup().await;
    let isolation =
        crate::codex_hooks::fail_next_intake_remove_sync(&intake, &accepted.occurrence).unwrap();
    gate.release();
    assert!(handler.await.unwrap().unwrap().into_parts().1.is_ok());
    server.settlements().wait_idle().await;
    peer.wait_for_exit().await.unwrap();

    assert!(!runtime.storage_failed().unwrap());
    assert!(isolation.exists());
    assert_eq!(
        outbox.load().unwrap().occurrence_receipt_count_for_test(),
        SINGLE_RECEIPT
    );
    assert_eq!(runtime.scan_unrouted_intake().unwrap(), Default::default());
    assert!(!isolation.exists());
    assert_cleanup_complete(&outbox, &intake, &accepted.occurrence);
    let retry = endpoint
        .retry_over_server(server.as_ref(), &accepted.occurrence, PeerEvidence::Direct)
        .await;
    assert!(retry.accepted);
    assert_eq!(retry.code, crate::codex_hooks::ACK_CODE_REPLAY);
    assert_eq!(retry.event_id, accepted.ack.event_id);
    assert_eq!(retry.sequence, accepted.ack.sequence);
    assert!(!runtime.storage_failed().unwrap());

    let next = fixture.prompt(SECOND_EVENT_SEQUENCE);
    let (handler, pending) = begin_event(&server, &endpoint, &next).await;
    assert!(pending.finish_accepted().await.ack.accepted);
    assert!(handler.await.unwrap().unwrap().into_parts().1.is_ok());
    drop(runtime);
    drop(server);
    endpoint.finish();
}

#[tokio::test]
async fn receipt_sweep_waits_for_live_unlink_durability() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(CLEANUP_OVERLAP_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let occurrence = fixture.start(FIRST_EVENT_SEQUENCE);
    let outbox = endpoint.outbox(&occurrence);
    let intake = intake(&endpoint, &occurrence);
    let response_gate = runtime.gate_next_response_for_test();
    let (handler, pending) = begin_event(&server, &endpoint, &occurrence).await;
    response_gate.wait_until_entered().await;
    let (accepted, peer) = pending.observe_accepted_pending_cleanup().await;
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (isolation, _hook) = crate::codex_hooks::gate_intake_remove_before_sync(
        &intake,
        &accepted.occurrence,
        entered_tx,
        release_rx,
    )
    .unwrap();
    response_gate.release();
    tokio::task::spawn_blocking(move || entered_rx.recv_timeout(RECOVERY_TIMEOUT))
        .await
        .unwrap()
        .unwrap();
    assert!(!isolation.exists());

    let (scan_tx, scan_rx) = mpsc::channel();
    let scan_runtime = Arc::clone(&runtime);
    let scan = std::thread::spawn(move || scan_tx.send(scan_runtime.scan_unrouted_intake()));
    assert!(matches!(
        scan_rx.recv_timeout(OVERLAP_TIMEOUT),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    assert_eq!(
        outbox.load().unwrap().occurrence_receipt_count_for_test(),
        SINGLE_RECEIPT
    );

    release_tx.send(()).unwrap();
    peer.wait_for_exit().await.unwrap();
    let _ = handler.await.unwrap().unwrap();
    assert_eq!(
        scan_rx.recv_timeout(RECOVERY_TIMEOUT).unwrap().unwrap(),
        Default::default()
    );
    scan.join().unwrap().unwrap();
    assert_cleanup_complete(&outbox, &intake, &accepted.occurrence);
    assert!(!runtime.storage_failed().unwrap());
    drop(runtime);
    drop(server);
    endpoint.finish();
}

async fn bound_runtime(
    endpoint: &IngressEndpoint,
    fixture: &Fixture,
) -> (Arc<Server>, Arc<Runtime>) {
    let supervisor = CodexHookIngressSupervisor::new();
    let server = Arc::new(endpoint.bind_server(&supervisor, fixture).await);
    let runtime = endpoint.bound_runtime(server.as_ref(), fixture).await;
    (server, runtime)
}

async fn begin_event(
    server: &Arc<Server>,
    endpoint: &IngressEndpoint,
    event: &HookEnvelope,
) -> (Handler, PendingIngressExchange) {
    let pending = endpoint.begin_exchange(server.socket_path(), event);
    let stream = server.accept().await.unwrap().0;
    let server = Arc::clone(server);
    let handler = tokio::spawn(async move { server.handle_stream(stream).await });
    (handler, pending)
}

fn intake(endpoint: &IngressEndpoint, occurrence: &HookEnvelope) -> CodexHookIntake {
    let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
        &endpoint.paths,
        occurrence.identity.terminal_id,
        occurrence.identity.runtime_epoch,
        occurrence.identity.invocation_id,
    )
    .unwrap();
    CodexHookIntake::for_identity(artifacts.intake_directory(), &occurrence.identity).unwrap()
}

fn assert_cleanup_complete(
    outbox: &CodexOutbox,
    intake: &CodexHookIntake,
    occurrence: &HookEnvelope,
) {
    assert!(outbox.load().unwrap().occurrence_receipt_count_for_test() == usize::default());
    assert!(intake.find_exact_occurrence(occurrence).unwrap().is_none());
}
