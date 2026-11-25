use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_live_source::PeerEvidence;
use super::ingress_test_support::{IngressEndpoint, PendingIngressExchange};
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture, SECOND_EVENT_SEQUENCE};
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::CodexHookIntake;
use std::sync::Arc;

const ACK_PERSIST_HOME: &str = "ingress-ack-persist-recovery-home";
const ACK_INDETERMINATE_HOME: &str = "ingress-ack-indeterminate-home";
const SINGLE_RECEIPT: usize = 1;

type Server = super::super::server::CodexHookServer;
type Runtime = super::super::runtime::CodexHookRuntime;
type Handler =
    tokio::task::JoinHandle<anyhow::Result<super::super::outcome::CodexHookHandleOutcome>>;

#[tokio::test]
async fn recoverable_ack_sync_failure_survives_response_loss_and_replay_cleanup() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(ACK_PERSIST_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let occurrence = fixture.start(FIRST_EVENT_SEQUENCE);
    let outbox = endpoint.outbox(&occurrence);
    crate::codex_hooks::fail_next_outbox_sync_after_snapshot_publication(outbox.path());
    let gate = runtime.gate_next_ack_write_for_test();

    let (handler, pending) = begin_event(&server, &endpoint, &occurrence).await;
    gate.wait_until_entered().await;
    let evidence = pending.abort_accepted_before_ack().await;
    evidence.assert_cleanup_pending();
    gate.release();
    assert!(handler.await.unwrap().unwrap().into_parts().1.is_err());
    server.settlements().wait_idle().await;
    evidence.assert_cleanup_pending();
    assert!(!runtime.storage_failed().unwrap());
    let assigned = assert_acknowledged_with_receipt(&outbox.load().unwrap());
    assert_raw(&endpoint, evidence.occurrence(), true);
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence,
        assigned.sequence
    );

    let retry = endpoint
        .retry_over_server(server.as_ref(), evidence.occurrence(), PeerEvidence::Direct)
        .await;
    assert!(retry.accepted);
    assert_eq!(retry.code, crate::codex_hooks::ACK_CODE_REPLAY);
    assert_eq!(retry.event_id, Some(assigned.event_id));
    assert_eq!(retry.sequence, Some(assigned.sequence.0));
    assert!(outbox.load().unwrap().occurrence_receipt_count_for_test() == usize::default());
    evidence.assert_cleanup_complete();
    assert_raw(&endpoint, evidence.occurrence(), false);
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
async fn acknowledgement_commit_indeterminate_closes_without_ack_and_latches_authority() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(ACK_INDETERMINATE_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let occurrence = fixture.start(FIRST_EVENT_SEQUENCE);
    let outbox = endpoint.outbox(&occurrence);
    crate::codex_hooks::fail_next_outbox_sync_after_snapshot_publication(outbox.path());
    let outbox_path = outbox.path().to_path_buf();
    crate::codex_hooks::set_outbox_reproof_hook(outbox_path.clone(), move || {
        std::fs::remove_file(outbox_path).unwrap();
    });

    let pending = endpoint.begin_exchange(server.socket_path(), &occurrence);
    let stream = server.accept().await.unwrap().0;
    let handled = tokio::spawn({
        let server = Arc::clone(&server);
        async move { server.handle_stream(stream).await }
    });
    let raw = pending.finish_indeterminate().await;
    assert!(handled.await.unwrap().unwrap().into_parts().1.is_err());
    server.settlements().wait_idle().await;

    assert!(runtime.storage_failed().unwrap());
    assert_eq!(runtime.state().unwrap(), None);
    assert_eq!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    );
    endpoint.retry_without_ack(server.as_ref(), &raw).await;

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

fn assert_acknowledged_with_receipt(state: &crate::codex_hooks::CodexOutboxState) -> HookEnvelope {
    assert!(state.pending_for_test().is_empty());
    assert_eq!(state.occurrence_receipt_count_for_test(), SINGLE_RECEIPT);
    state.acknowledged_events_for_test().back().unwrap().clone()
}

fn assert_raw(endpoint: &IngressEndpoint, occurrence: &HookEnvelope, retained: bool) {
    let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
        &endpoint.paths,
        occurrence.identity.terminal_id,
        occurrence.identity.runtime_epoch,
        occurrence.identity.invocation_id,
    )
    .unwrap();
    let intake =
        CodexHookIntake::for_identity(artifacts.intake_directory(), &occurrence.identity).unwrap();
    assert_eq!(
        intake.find_exact_occurrence(occurrence).unwrap().is_some(),
        retained
    );
}
