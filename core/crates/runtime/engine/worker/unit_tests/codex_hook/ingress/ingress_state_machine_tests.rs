use super::super::constants::{
    CODEX_INGRESS_ALREADY_RUNNING_ERROR, CODEX_INGRESS_START_CANCELLED_ERROR,
    CODEX_REQUEST_TIMEOUT, CODEX_SOCKET_PROBE_TIMEOUT,
};
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};

const CONCURRENT_HOME: &str = "ingress-concurrent-start-home";
const CANCELLED_HOME: &str = "ingress-cancelled-start-home";
const ABANDONED_CALLER_HOME: &str = "ingress-abandoned-caller-home";
const START_FINISHED_EARLY: &str = "gated Codex ingress start finished before release";

#[tokio::test]
async fn concurrent_start_fails_without_waiting_for_the_starting_bind() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(CONCURRENT_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let gate = supervisor.gate_next_start_for_test();
    {
        let first = endpoint.try_start(&supervisor, &fixture);
        tokio::pin!(first);

        tokio::select! {
            _ = gate.wait_until_entered() => {}
            _ = &mut first => panic!("{START_FINISHED_EARLY}"),
        }
        let duplicate_result = tokio::time::timeout(
            CODEX_SOCKET_PROBE_TIMEOUT,
            endpoint.try_start(&supervisor, &fixture),
        )
        .await
        .unwrap();
        let duplicate = match duplicate_result {
            Ok(_) => panic!("{START_FINISHED_EARLY}"),
            Err(error) => error,
        };

        assert!(
            duplicate
                .to_string()
                .contains(CODEX_INGRESS_ALREADY_RUNNING_ERROR)
        );
        gate.release();
        first.await.unwrap();
    }
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

#[tokio::test]
async fn shutdown_during_start_cancels_bind_and_leaves_a_reusable_idle_state() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(CANCELLED_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let gate = supervisor.gate_next_start_for_test();
    let (start_result, shutdown_result) = {
        let start = endpoint.try_start(&supervisor, &fixture);
        tokio::pin!(start);

        tokio::select! {
            _ = gate.wait_until_entered() => {}
            _ = &mut start => panic!("{START_FINISHED_EARLY}"),
        }
        tokio::join!(&mut start, supervisor.shutdown())
    };

    let start_error = match start_result {
        Ok(_) => panic!("{START_FINISHED_EARLY}"),
        Err(error) => error,
    };
    assert!(
        start_error
            .to_string()
            .contains(CODEX_INGRESS_START_CANCELLED_ERROR)
    );
    shutdown_result.unwrap();
    assert!(!socket.exists());
    endpoint.start(&supervisor, &fixture).await;
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

#[tokio::test]
async fn dropping_the_start_future_leaves_startup_in_supervisor_custody() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(ABANDONED_CALLER_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let gate = supervisor.gate_next_start_for_test();
    {
        let start = endpoint.try_start(&supervisor, &fixture);
        tokio::pin!(start);
        tokio::select! {
            _ = gate.wait_until_entered() => {}
            _ = &mut start => panic!("{START_FINISHED_EARLY}"),
        }
    }

    gate.release();
    tokio::time::timeout(CODEX_REQUEST_TIMEOUT, async {
        while !socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    endpoint.probe_listener(&socket).await;
    assert!(
        endpoint
            .exchange(&socket, &fixture.start(FIRST_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}
