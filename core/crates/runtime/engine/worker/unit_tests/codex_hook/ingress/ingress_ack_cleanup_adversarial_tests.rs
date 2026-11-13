use super::super::constants::CODEX_ACK_IDENTITY;
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_live_source::PeerEvidence;
use super::ingress_test_support::{IngressEndpoint, PendingIngressExchange};
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use crate::agent::hooks::{HookEnvelope, HookProviderKind};
use std::sync::Arc;

const ACK_ABORT_HOME: &str = "ingress-ack-abort-cleanup-home";
const RECEIPT_FAULT_HOME: &str = "ingress-receipt-cleanup-fault-home";
const RESPONSE_LOSS_HOME: &str = "ingress-response-loss-retry-home";
const POISONED_RESPONSE_LOSS_HOME: &str = "ingress-poisoned-response-loss-retry-home";

type Server = super::super::server::CodexHookServer;
type Handler =
    tokio::task::JoinHandle<anyhow::Result<super::super::outcome::CodexHookHandleOutcome>>;

#[tokio::test]
async fn observed_negative_ack_survives_abort_before_raw_cleanup() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(ACK_ABORT_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let gate = runtime.gate_next_response_for_test();
    let event = rejected_event(&fixture);
    let (handler, pending) = begin_event(&server, &endpoint, &event).await;
    gate.wait_until_entered().await;

    let mut evidence = pending.observe_durable_ack().await;
    assert_eq!(evidence.ack.as_ref().unwrap().code, CODEX_ACK_IDENTITY);
    evidence.assert_cleanup_pending();
    handler.abort();
    assert!(matches!(handler.await, Err(error) if error.is_cancelled()));
    server.settlements().wait_idle().await;
    evidence.finish_peer().await;

    evidence.assert_cleanup_pending();
    assert!(!runtime.storage_failed().unwrap());
    drop(runtime);
    drop(server);
    let (recovered, replay) = endpoint.restart_runtime(&fixture);
    assert_eq!(replay, Default::default());
    evidence.assert_cleanup_pending();
    drop(recovered);
    endpoint.finish();
}

#[tokio::test]
async fn raw_completion_fault_retains_receipt_for_cold_recovery() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RECEIPT_FAULT_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let gate = runtime.gate_next_response_for_test();
    let event = rejected_event(&fixture);
    let (handler, pending) = begin_event(&server, &endpoint, &event).await;
    gate.wait_until_entered().await;
    let mut evidence = pending.observe_durable_ack().await;
    evidence.assert_cleanup_pending();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    crate::codex_hooks::set_outbox_commit_hook(evidence.outbox_path(), move || {
        let _ = entered_tx.send(());
        std::panic::panic_any(());
    });

    gate.release();
    entered_rx.await.unwrap();
    let _ = handler.await;
    server.settlements().wait_idle().await;
    evidence.finish_peer().await;
    evidence.assert_raw_removed_receipt_retained();
    drop(runtime);
    drop(server);

    let (recovered, replay) = endpoint.restart_runtime(&fixture);
    assert_eq!(replay, Default::default());
    evidence.assert_raw_removed_receipt_retained();
    assert_eq!(
        recovered.scan_unrouted_intake().unwrap(),
        Default::default()
    );
    evidence.assert_cleanup_complete();
    drop(recovered);
    endpoint.finish();
}

#[tokio::test]
async fn lost_negative_ack_response_retries_with_stable_identity_and_code() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RESPONSE_LOSS_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let gate = runtime.gate_next_ack_write_for_test();
    let event = rejected_event(&fixture);
    let (handler, pending) = begin_event(&server, &endpoint, &event).await;
    gate.wait_until_entered().await;
    let evidence = pending.abort_rejected_before_ack().await;
    evidence.assert_cleanup_pending();

    gate.release();
    let outcome = handler.await.unwrap().unwrap();
    assert!(outcome.into_parts().1.is_err());
    server.settlements().wait_idle().await;
    evidence.assert_cleanup_pending();
    assert!(!runtime.storage_failed().unwrap());

    let hot_retry = endpoint
        .retry_over_server(server.as_ref(), evidence.occurrence(), PeerEvidence::Direct)
        .await;
    assert!(!hot_retry.accepted);
    assert_eq!(hot_retry.event_id, Some(evidence.event.event_id));
    assert_eq!(hot_retry.sequence, Some(evidence.event.sequence.0));
    assert_eq!(hot_retry.code, CODEX_ACK_IDENTITY);
    evidence.assert_cleanup_complete();
    assert!(!runtime.storage_failed().unwrap());
    drop(runtime);
    drop(server);
    endpoint.finish();
}

#[tokio::test]
async fn poisoned_retry_preserves_negative_ack_until_cold_recovery() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(POISONED_RESPONSE_LOSS_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let gate = runtime.gate_next_ack_write_for_test();
    let event = rejected_event(&fixture);
    let (handler, pending) = begin_event(&server, &endpoint, &event).await;
    gate.wait_until_entered().await;
    let evidence = pending.abort_rejected_before_ack().await;
    gate.release();
    let outcome = handler.await.unwrap().unwrap();
    assert!(outcome.into_parts().1.is_err());
    server.settlements().wait_idle().await;
    evidence.assert_cleanup_pending();

    runtime.poison_lifecycle_for_test();
    let poisoned_retry = endpoint
        .retry_over_server(server.as_ref(), evidence.occurrence(), PeerEvidence::Direct)
        .await;
    assert!(!poisoned_retry.accepted);
    assert_eq!(poisoned_retry.event_id, Some(evidence.event.event_id));
    assert_eq!(poisoned_retry.sequence, Some(evidence.event.sequence.0));
    assert_eq!(poisoned_retry.code, CODEX_ACK_IDENTITY);
    evidence.assert_cleanup_pending();
    drop(runtime);
    drop(server);

    let (retry_server, recovered) = bound_runtime(&endpoint, &fixture).await;
    let retry = endpoint
        .retry_over_server(
            retry_server.as_ref(),
            evidence.occurrence(),
            PeerEvidence::Direct,
        )
        .await;
    assert!(!retry.accepted);
    assert_eq!(retry.event_id, Some(evidence.event.event_id));
    assert_eq!(retry.sequence, Some(evidence.event.sequence.0));
    assert_eq!(retry.code, CODEX_ACK_IDENTITY);
    evidence.assert_cleanup_complete();
    drop(recovered);
    drop(retry_server);
    endpoint.finish();
}

async fn bound_runtime(
    endpoint: &IngressEndpoint,
    fixture: &Fixture,
) -> (Arc<Server>, Arc<super::super::runtime::CodexHookRuntime>) {
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

fn rejected_event(fixture: &Fixture) -> HookEnvelope {
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.provider = HookProviderKind::Claude;
    event
}
