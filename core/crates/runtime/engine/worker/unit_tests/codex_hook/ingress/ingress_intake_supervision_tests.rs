use super::super::cancellation::IngressCancellation;
use super::super::constants::{
    CODEX_INTAKE_ENUMERATION_ERROR, CODEX_INTAKE_TASK_ERROR, CODEX_REQUEST_TIMEOUT,
    CODEX_SOCKET_PROBE_TIMEOUT,
};
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::Fixture;
use crate::local_unix::LocalUnixStream;
use tokio::task::JoinSet;

const INTAKE_FAILURE_HOME: &str = "ingress-intake-failure-home";
const INTAKE_PANIC_HOME: &str = "ingress-intake-panic-home";
const INTAKE_PANIC_DETAIL: &str = "injected durable intake panic";

#[tokio::test]
async fn intake_authority_failure_stops_the_listener_fail_closed() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(INTAKE_FAILURE_HOME);
    let supervisor = super::super::ingress_supervisor::CodexHookIngressSupervisor::new();
    let server = endpoint.bind_server(&supervisor, &fixture).await;
    let socket = server.socket_path().to_path_buf();
    server.router().poison_lock_for_test();

    let result = tokio::time::timeout(
        CODEX_REQUEST_TIMEOUT,
        super::super::ingress_task::run(server, IngressCancellation::default()),
    )
    .await
    .unwrap();
    let detail = format!("{:#}", result.unwrap_err());

    assert!(detail.contains(CODEX_INTAKE_ENUMERATION_ERROR));
    assert!(detail.contains(CODEX_INTAKE_TASK_ERROR));
    assert!(LocalUnixStream::connect(&socket).await.is_err());
    endpoint.finish();
}

#[tokio::test]
async fn intake_panic_drains_connections_and_waits_for_blocking_custody() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(INTAKE_PANIC_HOME);
    let supervisor = super::super::ingress_supervisor::CodexHookIngressSupervisor::new();
    let server = endpoint.bind_server(&supervisor, &fixture).await;
    let socket = server.socket_path().to_path_buf();
    let settlements = server.settlements();
    let cancellation = IngressCancellation::default();
    let cancelled = cancellation.clone();
    let (release_connection, connection_released) = tokio::sync::oneshot::channel();
    let mut connections = JoinSet::new();
    connections.spawn(async move {
        let _ = connection_released.await;
    });
    let mut running = tokio::spawn(super::super::ingress_task::run_with_tasks_for_test(
        server,
        cancellation,
        async { panic!("{INTAKE_PANIC_DETAIL}") },
        connections,
    ));

    tokio::time::timeout(CODEX_REQUEST_TIMEOUT, cancelled.cancelled())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(CODEX_SOCKET_PROBE_TIMEOUT, &mut running)
            .await
            .is_err()
    );
    let settlement = settlements.admit();
    release_connection.send(()).unwrap();
    assert!(
        tokio::time::timeout(CODEX_SOCKET_PROBE_TIMEOUT, &mut running)
            .await
            .is_err()
    );
    drop(settlement);
    let result = tokio::time::timeout(CODEX_REQUEST_TIMEOUT, running)
        .await
        .unwrap()
        .unwrap();
    let detail = format!("{:#}", result.unwrap_err());

    assert!(detail.contains(CODEX_INTAKE_TASK_ERROR));
    assert!(detail.contains(INTAKE_PANIC_DETAIL));
    assert!(LocalUnixStream::connect(&socket).await.is_err());
    endpoint.finish();
}
