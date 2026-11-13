use super::super::constants::CODEX_INGRESS_ALREADY_RUNNING_ERROR;
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture, SECOND_EVENT_SEQUENCE};
use crate::local_unix::LocalUnixStream;
use std::time::Duration;

const LIFECYCLE_HOME: &str = "ingress-lifecycle-home";
const BLOCKED_HOME: &str = "ingress-blocked-home";
const SETTLEMENT_HOME: &str = "ingress-settlement-home";
const RUNNING_HOME: &str = "ingress-running-home";
const DUPLICATE_HOME: &str = "ingress-duplicate-home";
const MALFORMED_HOME: &str = "ingress-malformed-home";
const DROP_HOME: &str = "ingress-drop-home";
const MALFORMED_REQUEST: &[u8] = b"{";
const ASYNC_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(1);

#[tokio::test]
async fn ingress_serves_hooks_and_restarts_after_idempotent_shutdown() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(LIFECYCLE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;

    assert!(
        endpoint
            .exchange(&socket, &fixture.start(FIRST_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    assert!(!socket.exists());
    supervisor.shutdown().await.unwrap();

    endpoint.start(&supervisor, &fixture).await;
    assert!(
        endpoint
            .exchange(&socket, &fixture.prompt(SECOND_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

#[tokio::test(start_paused = true)]
async fn shutdown_drains_a_client_blocked_while_reading_its_request() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(BLOCKED_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;
    let mut client = LocalUnixStream::connect(&socket).await.unwrap();

    tokio::task::yield_now().await;
    supervisor.shutdown().await.unwrap();
    assert!(
        super::transport_support::try_read_ack(&mut client)
            .await
            .is_err()
    );
    assert!(!socket.exists());
    endpoint.finish();
}

#[tokio::test]
async fn shutdown_preserves_an_accepted_ack_until_handoff_settles() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(SETTLEMENT_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let response_gate = runtime.gate_next_response_for_test();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    let exchange = endpoint.begin_exchange(&socket, &event);
    response_gate.wait_until_entered().await;
    let (accepted, peer) = exchange.observe_accepted_pending_cleanup().await;
    let mut shutdown = Box::pin(supervisor.shutdown());

    assert!(
        tokio::time::timeout(
            super::super::constants::CODEX_SOCKET_PROBE_TIMEOUT,
            &mut shutdown,
        )
        .await
        .is_err()
    );
    assert!(socket.exists());
    response_gate.release();
    shutdown.await.unwrap();
    peer.wait_for_exit().await.unwrap();

    assert!(!socket.exists());
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence.0,
        accepted.ack.sequence.unwrap()
    );
    endpoint.finish();
}

#[tokio::test]
async fn duplicate_start_does_not_replace_the_running_ingress() {
    let fixture = Fixture::new();
    let running = IngressEndpoint::new(RUNNING_HOME);
    let duplicate = IngressEndpoint::new(DUPLICATE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let running_socket = running.socket_path();
    running.start(&supervisor, &fixture).await;

    let duplicate_start = duplicate.try_start(&supervisor, &fixture).await;
    assert!(duplicate_start.is_err());
    let error = duplicate_start.err().unwrap();

    assert!(
        error
            .to_string()
            .contains(CODEX_INGRESS_ALREADY_RUNNING_ERROR)
    );
    assert!(!duplicate.socket_path().exists());
    assert!(
        running
            .exchange(&running_socket, &fixture.start(FIRST_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    running.finish();
    duplicate.finish();
}

#[tokio::test]
async fn malformed_connection_does_not_stop_the_listener() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(MALFORMED_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;
    let mut malformed = LocalUnixStream::connect(&socket).await.unwrap();
    super::transport_support::write_bytes_frame(&mut malformed, MALFORMED_REQUEST).await;

    assert!(
        !super::transport_support::read_ack(&mut malformed)
            .await
            .accepted
    );
    assert!(
        endpoint
            .exchange(&socket, &fixture.start(FIRST_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

#[tokio::test]
async fn dropping_the_supervisor_revokes_listener_custody() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(DROP_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;

    drop(supervisor);
    tokio::time::timeout(ASYNC_SETTLEMENT_TIMEOUT, async {
        while socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    endpoint.finish();
}
