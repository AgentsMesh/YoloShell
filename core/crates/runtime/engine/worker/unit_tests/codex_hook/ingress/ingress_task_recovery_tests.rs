use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};

const TASK_RECOVERY_HOME: &str = "ingress-task-recovery-home";
const NEXT_LISTENER_PUBLICATION: u64 = 1;

#[tokio::test]
async fn an_aborted_listener_is_automatically_rebound_without_renumbering_routes() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(TASK_RECOVERY_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    endpoint.start(&supervisor, &fixture).await;
    let initial_revision = supervisor
        .subscribe_projection_routes()
        .unwrap()
        .snapshot
        .first()
        .unwrap()
        .revision;

    let publications = supervisor.listener_publications_for_test();
    assert!(supervisor.abort_listener_for_test());
    tokio::time::timeout(
        super::super::constants::CODEX_REQUEST_TIMEOUT,
        supervisor.wait_for_listener_publications_for_test(
            publications.saturating_add(NEXT_LISTENER_PUBLICATION),
        ),
    )
    .await
    .unwrap();
    let restarted_revision = supervisor
        .subscribe_projection_routes()
        .unwrap()
        .snapshot
        .first()
        .unwrap()
        .revision;
    assert_eq!(restarted_revision, initial_revision);

    assert!(
        endpoint
            .exchange(&socket, &fixture.start(FIRST_EVENT_SEQUENCE))
            .await
            .accepted
    );
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}
