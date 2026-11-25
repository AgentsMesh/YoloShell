use super::super::constants::{
    CODEX_INGRESS_DRAIN_TIMEOUT, CODEX_INGRESS_RESTART_MAX_DELAY, CODEX_REQUEST_TIMEOUT,
};
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const REPEATED_FAILURE_HOME: &str = "ingress-repeated-rebind-home";
const SHUTDOWN_RACE_HOME: &str = "ingress-rebind-shutdown-home";
const RECOVERY_ORDER_HOME: &str = "ingress-rebind-recovery-home";
const REBIND_FAILURE_COUNT: usize = 8;
const LISTENER_FAILURE_COUNT: usize = 1;
const EXPECTED_RESTART_DELAY_COUNT: usize =
    REBIND_FAILURE_COUNT.saturating_add(LISTENER_FAILURE_COUNT);
const NEXT_LISTENER_PUBLICATION: u64 = 1;

#[tokio::test(start_paused = true)]
async fn repeated_rebind_failure_uses_capped_backoff_then_recovers() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(REPEATED_FAILURE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;
    let publications = supervisor.listener_publications_for_test();

    supervisor.fail_rebinds_for_test(REBIND_FAILURE_COUNT);
    supervisor.fail_listener_for_test();
    tokio::time::timeout(
        CODEX_INGRESS_DRAIN_TIMEOUT,
        supervisor.wait_for_restart_delays_for_test(EXPECTED_RESTART_DELAY_COUNT),
    )
    .await
    .unwrap();
    tokio::time::timeout(
        CODEX_INGRESS_DRAIN_TIMEOUT,
        supervisor.wait_for_listener_publications_for_test(
            publications.saturating_add(NEXT_LISTENER_PUBLICATION),
        ),
    )
    .await
    .unwrap();

    let delays = supervisor.restart_delays_for_test();
    assert_eq!(delays.len(), EXPECTED_RESTART_DELAY_COUNT);
    assert!(
        delays
            .iter()
            .all(|delay| *delay <= CODEX_INGRESS_RESTART_MAX_DELAY)
    );
    assert!(delays.contains(&CODEX_INGRESS_RESTART_MAX_DELAY));
    assert!(
        endpoint
            .exchange(&socket, &fixture.start(FIRST_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

#[tokio::test(start_paused = true)]
async fn shutdown_during_restart_backoff_prevents_socket_republication() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(SHUTDOWN_RACE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;
    let publications = supervisor.listener_publications_for_test();
    let delay_target = supervisor
        .restart_delays_for_test()
        .len()
        .saturating_add(LISTENER_FAILURE_COUNT);

    supervisor.fail_listener_for_test();
    supervisor
        .wait_for_restart_delays_for_test(delay_target)
        .await;
    supervisor.shutdown().await.unwrap();
    tokio::time::advance(CODEX_INGRESS_RESTART_MAX_DELAY).await;
    tokio::task::yield_now().await;

    assert_eq!(supervisor.listener_publications_for_test(), publications);
    assert!(!socket.exists());
    endpoint.finish();
}

#[tokio::test]
async fn automatic_rebind_runs_recovery_before_republishing_the_socket() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RECOVERY_ORDER_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;
    let publications = supervisor.listener_publications_for_test();
    let recovery_ran = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&recovery_ran);
    let observed_socket = socket.clone();
    super::super::recovery_test_hook::set(move || {
        assert!(!observed_socket.exists());
        observed.store(true, Ordering::SeqCst);
    });

    supervisor.fail_listener_for_test();
    tokio::time::timeout(
        CODEX_REQUEST_TIMEOUT,
        supervisor.wait_for_listener_publications_for_test(
            publications.saturating_add(NEXT_LISTENER_PUBLICATION),
        ),
    )
    .await
    .unwrap();

    assert!(recovery_ran.load(Ordering::SeqCst));
    assert!(socket.exists());
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}
