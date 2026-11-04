use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_live_source::PeerEvidence;
use super::ingress_test_support::{CleanupEvidence, IngressEndpoint, PendingIngressExchange};
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture, SECOND_EVENT_SEQUENCE};
use crate::agent::hooks::HookEnvelope;
use std::sync::Arc;

const HOT_RETRY_HOME: &str = "ingress-accepted-ack-hot-retry-home";
const COLD_RETRY_HOME: &str = "ingress-accepted-ack-cold-retry-home";

type Server = super::super::server::CodexHookServer;
type Runtime = super::super::runtime::CodexHookRuntime;
type Handler =
    tokio::task::JoinHandle<anyhow::Result<super::super::outcome::CodexHookHandleOutcome>>;

#[tokio::test]
async fn lost_accepted_ack_replays_and_cleans_in_the_same_runtime() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(HOT_RETRY_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let evidence = lose_accepted_ack(&server, &runtime, &endpoint, &fixture).await;

    let retry = endpoint
        .retry_over_server(server.as_ref(), evidence.occurrence(), PeerEvidence::Direct)
        .await;
    assert!(retry.accepted);
    assert_eq!(retry.code, crate::codex_hooks::ACK_CODE_REPLAY);
    assert_eq!(retry.event_id, Some(evidence.event.event_id));
    assert_eq!(retry.sequence, Some(evidence.event.sequence.0));
    evidence.assert_cleanup_complete();
    assert!(!runtime.storage_failed().unwrap());

    let next = fixture.prompt(SECOND_EVENT_SEQUENCE);
    let (handler, pending) = begin_event(&server, &endpoint, &next).await;
    let accepted = pending.finish_accepted().await;
    assert!(handler.await.unwrap().unwrap().into_parts().1.is_ok());
    assert_eq!(accepted.ack.code, crate::codex_hooks::ACK_CODE_ACCEPTED);
    drop(runtime);
    drop(server);
    endpoint.finish();
}

#[tokio::test]
async fn lost_accepted_ack_replays_with_stable_identity_after_cold_restart() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(COLD_RETRY_HOME);
    let (server, runtime) = bound_runtime(&endpoint, &fixture).await;
    let evidence = lose_accepted_ack(&server, &runtime, &endpoint, &fixture).await;
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
    assert!(retry.accepted);
    assert_eq!(retry.code, crate::codex_hooks::ACK_CODE_REPLAY);
    assert_eq!(retry.event_id, Some(evidence.event.event_id));
    assert_eq!(retry.sequence, Some(evidence.event.sequence.0));
    evidence.assert_cleanup_complete();
    assert!(!recovered.storage_failed().unwrap());
    drop(recovered);
    drop(retry_server);
    endpoint.finish();
}

async fn lose_accepted_ack(
    server: &Arc<Server>,
    runtime: &Arc<Runtime>,
    endpoint: &IngressEndpoint,
    fixture: &Fixture,
) -> CleanupEvidence {
    let gate = runtime.gate_next_ack_write_for_test();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    let (handler, pending) = begin_event(server, endpoint, &event).await;
    gate.wait_until_entered().await;
    let evidence = pending.abort_accepted_before_ack().await;
    evidence.assert_cleanup_pending();
    gate.release();
    let outcome = handler.await.unwrap().unwrap();
    assert!(outcome.into_parts().1.is_err());
    server.settlements().wait_idle().await;
    evidence.assert_cleanup_pending();
    assert!(!runtime.storage_failed().unwrap());
    evidence
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
