use super::super::constants::{CODEX_INGRESS_DRAIN_TIMEOUT, CODEX_INGRESS_SHUTDOWN_ERROR};
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture, SECOND_EVENT_SEQUENCE};
use crate::agent::hooks::HookEnvelope;
use std::sync::Arc;

const GENERATION_FENCE_HOME: &str = "ingress-generation-fence-home";
const ACK_FENCE_HOME: &str = "ingress-ack-fence-home";
const SETTLEMENT_FENCE_HOME: &str = "ingress-settlement-fence-home";

#[tokio::test(start_paused = true)]
async fn shutdown_timeout_revokes_late_blocking_ingest_and_allows_restart() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(GENERATION_FENCE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let first_runtime = endpoint.start(&supervisor, &fixture).await;
    let first_authority = first_runtime.subscribe_authority();
    let gate = endpoint.gate_next_process_scan();
    let first = fixture.start(FIRST_EVENT_SEQUENCE);
    let mut exchange = endpoint.begin_exchange(&socket, &first);
    let entered = Arc::clone(&gate);
    tokio::task::spawn_blocking(move || entered.wait_until_entered())
        .await
        .unwrap();
    assert!(exchange.is_running());

    let mut shutdown = Box::pin(supervisor.shutdown());
    assert!(futures::poll!(&mut shutdown).is_pending());
    tokio::time::advance(CODEX_INGRESS_DRAIN_TIMEOUT).await;
    tokio::task::yield_now().await;
    let shutdown_error = shutdown.await.unwrap_err();

    assert!(
        shutdown_error
            .to_string()
            .contains(CODEX_INGRESS_SHUTDOWN_ERROR)
    );
    assert!(!socket.exists());
    let stale = exchange.finish_without_ack().await;
    discard_occurrence(&endpoint, &stale);

    let second_runtime = endpoint.start(&supervisor, &fixture).await;
    let second_authority = second_runtime.subscribe_authority();
    let second = endpoint
        .exchange_detailed(&socket, &fixture.start(SECOND_EVENT_SEQUENCE))
        .await;
    let stable_outbox = endpoint.outbox(&second.occurrence).load().unwrap();
    let stable_snapshot = std::fs::read(second_runtime.snapshot_path()).unwrap();
    let stable_state = second_runtime.state().unwrap();
    let stable_authority = second_authority.borrow().clone();
    let stale_authority = first_authority.borrow().clone();

    gate.release();
    let returned = Arc::clone(&gate);
    tokio::task::spawn_blocking(move || returned.wait_until_returned())
        .await
        .unwrap();
    tokio::task::yield_now().await;

    let settled_outbox = endpoint.outbox(&second.occurrence).load().unwrap();
    assert_eq!(settled_outbox, stable_outbox);
    assert!(
        settled_outbox
            .acknowledged_events_for_test()
            .iter()
            .all(|event| event.event_id != stale.event_id)
    );
    assert_eq!(
        std::fs::read(second_runtime.snapshot_path()).unwrap(),
        stable_snapshot
    );
    assert_eq!(second_runtime.state().unwrap(), stable_state);
    assert_eq!(*second_authority.borrow(), stable_authority);
    assert_eq!(*first_authority.borrow(), stale_authority);
    assert!(!first_runtime.storage_failed().unwrap());

    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

#[tokio::test]
async fn cancellation_between_outcome_and_ack_blocks_old_generation_ack() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(ACK_FENCE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let gate = runtime.gate_next_ack_write_for_test();
    let exchange = endpoint.begin_exchange(&socket, &fixture.start(FIRST_EVENT_SEQUENCE));
    gate.wait_until_entered().await;

    let mut shutdown = Box::pin(supervisor.shutdown());
    assert!(futures::poll!(&mut shutdown).is_pending());
    gate.release();
    let committed_sequence = exchange.finish_aborted().await;
    shutdown.await.unwrap();

    assert!(!socket.exists());
    assert_eq!(committed_sequence.0, FIRST_EVENT_SEQUENCE);
    endpoint.finish();
}

#[tokio::test]
async fn committed_settlement_survives_cancellation_before_settlement() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(SETTLEMENT_FENCE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let gate = runtime.gate_next_commit_settlement_for_test();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    let exchange = endpoint.begin_exchange(&socket, &event);
    let entered = Arc::clone(&gate);
    tokio::task::spawn_blocking(move || entered.wait_until_entered())
        .await
        .unwrap();

    let mut shutdown = Box::pin(supervisor.shutdown());
    assert!(futures::poll!(&mut shutdown).is_pending());
    gate.release();
    let committed_sequence = exchange.finish_aborted().await;
    shutdown.await.unwrap();

    assert_eq!(committed_sequence.0, FIRST_EVENT_SEQUENCE);
    let state = endpoint.outbox(&event).load().unwrap();
    assert!(state.pending_for_test().is_empty());
    assert_eq!(state.acknowledged_sequence(), FIRST_EVENT_SEQUENCE);
    assert!(!runtime.storage_failed().unwrap());
    endpoint.finish();
}

fn discard_occurrence(endpoint: &IngressEndpoint, occurrence: &HookEnvelope) {
    let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
        &endpoint.paths,
        occurrence.identity.terminal_id,
        occurrence.identity.runtime_epoch,
        occurrence.identity.invocation_id,
    )
    .unwrap();
    let intake = crate::codex_hooks::CodexHookIntake::for_identity(
        artifacts.intake_directory(),
        &occurrence.identity,
    )
    .unwrap();
    let durable = intake.find_exact_occurrence(occurrence).unwrap().unwrap();
    intake.complete(&durable).unwrap();
}
