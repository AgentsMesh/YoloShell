use super::super::constants::CODEX_INGRESS_CONNECTION_DRAIN_TIMEOUT;
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use std::sync::Arc;

const BLOCKING_CUSTODY_HOME: &str = "ingress-blocking-custody-home";

#[tokio::test(start_paused = true)]
async fn shutdown_waits_for_blocking_ingest_after_connection_abort() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(BLOCKING_CUSTODY_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let authority = runtime.subscribe_authority();
    let gate = runtime.gate_next_handoff_for_test();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    let mut exchange = endpoint.begin_exchange(&socket, &event);
    let entered = Arc::clone(&gate);
    tokio::task::spawn_blocking(move || entered.wait_until_entered())
        .await
        .unwrap();
    let helper_was_running = exchange.is_running();
    let committed_sequence = exchange.assert_committed();
    let mut shutdown = Box::pin(supervisor.shutdown());
    let mut shutdown_result = None;
    let shutdown_started = match futures::poll!(&mut shutdown) {
        std::task::Poll::Pending => true,
        std::task::Poll::Ready(result) => {
            shutdown_result = Some(result);
            false
        }
    };
    tokio::task::yield_now().await;
    tokio::time::advance(CODEX_INGRESS_CONNECTION_DRAIN_TIMEOUT).await;
    tokio::task::yield_now().await;
    let shutdown_waited_for_custody = if shutdown_started {
        match futures::poll!(&mut shutdown) {
            std::task::Poll::Pending => true,
            std::task::Poll::Ready(result) => {
                shutdown_result = Some(result);
                false
            }
        }
    } else {
        false
    };
    let socket_was_retained = socket.exists();

    gate.release();
    let shutdown_result = match shutdown_result {
        Some(result) => result,
        None => shutdown.await,
    };
    let retained_sequence = exchange.finish_drained_without_ack().await;
    let shutdown_succeeded = shutdown_result.is_ok();
    let socket_was_removed = !socket.exists();
    assert!(shutdown_started);
    assert!(helper_was_running);
    assert!(shutdown_waited_for_custody);
    assert!(socket_was_retained);
    assert!(shutdown_succeeded, "{shutdown_result:?}");
    assert!(socket_was_removed);
    assert_eq!(retained_sequence, committed_sequence);
    assert!(!runtime.storage_failed().unwrap());
    assert!(runtime.state().unwrap().is_some());
    assert!(!matches!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    ));
    endpoint.finish();
}
