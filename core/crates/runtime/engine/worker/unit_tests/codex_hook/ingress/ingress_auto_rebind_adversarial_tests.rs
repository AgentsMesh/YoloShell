use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};

const ERROR_HOME: &str = "ingress-error-rebind-home";
const PANIC_HOME: &str = "ingress-panic-rebind-home";
const NEXT_LISTENER_PUBLICATION: u64 = 1;

enum ListenerFailure {
    Error,
    Panic,
}

#[tokio::test]
async fn listener_error_is_contained_and_automatically_rebound() {
    assert_automatic_rebind(ERROR_HOME, ListenerFailure::Error).await;
}

#[tokio::test]
async fn listener_panic_is_contained_and_automatically_rebound() {
    assert_automatic_rebind(PANIC_HOME, ListenerFailure::Panic).await;
}

async fn assert_automatic_rebind(home: &str, failure: ListenerFailure) {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(home);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;
    let initial_revision = route_revision(&supervisor);
    let publications = supervisor.listener_publications_for_test();

    match failure {
        ListenerFailure::Error => supervisor.fail_listener_for_test(),
        ListenerFailure::Panic => supervisor.panic_listener_for_test(),
    }
    tokio::time::timeout(
        super::super::constants::CODEX_REQUEST_TIMEOUT,
        supervisor.wait_for_listener_publications_for_test(
            publications.saturating_add(NEXT_LISTENER_PUBLICATION),
        ),
    )
    .await
    .unwrap();

    assert_eq!(route_revision(&supervisor), initial_revision);
    assert!(
        endpoint
            .exchange(&socket, &fixture.start(FIRST_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

fn route_revision(supervisor: &CodexHookIngressSupervisor) -> u64 {
    supervisor
        .subscribe_projection_routes()
        .unwrap()
        .snapshot
        .first()
        .unwrap()
        .revision
}
