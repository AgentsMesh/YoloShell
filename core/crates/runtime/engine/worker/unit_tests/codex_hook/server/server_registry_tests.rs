use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::super::publication::CodexAuthorityView;
use super::ingress_test_support::IngressEndpoint;
use super::ingress_test_support::committed_runtime;
use super::native_ingress_support::Fixture;

const REPLACEMENT_HOME_DIRECTORY: &str = "registry-replacement-home";
const FIRST_WORKER_HOME_DIRECTORY: &str = "registry-first-worker-home";
const SECOND_WORKER_HOME_DIRECTORY: &str = "registry-second-worker-home";

#[tokio::test]
async fn server_replacement_recovers_route_without_another_begin() {
    let fixture = Fixture::new();
    let supervisor = CodexHookIngressSupervisor::new();
    let endpoint = IngressEndpoint::new(REPLACEMENT_HOME_DIRECTORY);
    let first = endpoint.bind_server(&supervisor, &fixture).await;
    let predecessor = endpoint.bound_runtime(&first, &fixture).await;
    assert_active(&predecessor);
    drop(first);

    let successor = endpoint.bind_server(&supervisor, &fixture).await;
    let successor_runtime = committed_runtime(successor.router(), &fixture);

    assert_superseded(&predecessor);
    assert_active(&successor_runtime);
    drop((successor, successor_runtime, predecessor, supervisor));
    endpoint.finish();
}

#[tokio::test]
async fn new_worker_registry_recovers_route_without_another_begin() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(REPLACEMENT_HOME_DIRECTORY);
    let first_supervisor = CodexHookIngressSupervisor::new();
    let first = endpoint.bind_server(&first_supervisor, &fixture).await;
    let first_runtime = endpoint.bound_runtime(&first, &fixture).await;
    drop((first, first_runtime, first_supervisor));

    let successor_supervisor = CodexHookIngressSupervisor::new();
    let successor = endpoint.bind_server(&successor_supervisor, &fixture).await;
    let recovered = committed_runtime(successor.router(), &fixture);

    assert_active(&recovered);
    drop((successor, recovered, successor_supervisor));
    endpoint.finish();
}

#[tokio::test]
async fn live_server_blocks_replacement_without_losing_authority() {
    let fixture = Fixture::new();
    let supervisor = CodexHookIngressSupervisor::new();
    let endpoint = IngressEndpoint::new(REPLACEMENT_HOME_DIRECTORY);
    let first = endpoint.bind_server(&supervisor, &fixture).await;
    let predecessor = endpoint.bound_runtime(&first, &fixture).await;

    assert!(
        endpoint
            .try_bind_server(&supervisor, &fixture)
            .await
            .is_err()
    );
    assert_active(&predecessor);

    drop((first, predecessor, supervisor));
    endpoint.finish();
}

#[tokio::test]
async fn identical_authority_keys_are_isolated_between_worker_supervisors() {
    let fixture = Fixture::new();
    let first_supervisor = CodexHookIngressSupervisor::new();
    let second_supervisor = CodexHookIngressSupervisor::new();
    let first_endpoint = IngressEndpoint::new(FIRST_WORKER_HOME_DIRECTORY);
    let second_endpoint = IngressEndpoint::new(SECOND_WORKER_HOME_DIRECTORY);
    let first = first_endpoint
        .bind_server(&first_supervisor, &fixture)
        .await;
    let isolated = second_endpoint
        .bind_server(&second_supervisor, &fixture)
        .await;
    let first_runtime = first_endpoint.bound_runtime(&first, &fixture).await;
    let isolated_runtime = second_endpoint.bound_runtime(&isolated, &fixture).await;
    drop(first);

    let successor = first_endpoint
        .bind_server(&first_supervisor, &fixture)
        .await;
    let successor_runtime = first_endpoint.bound_runtime(&successor, &fixture).await;

    assert_superseded(&first_runtime);
    assert_active(&successor_runtime);
    assert_active(&isolated_runtime);
    drop((successor, isolated, successor_runtime));
    drop((first_runtime, isolated_runtime));
    drop((first_supervisor, second_supervisor));
    first_endpoint.finish();
    second_endpoint.finish();
}

#[tokio::test]
async fn dropping_one_worker_supervisor_does_not_affect_another_worker() {
    let fixture = Fixture::new();
    let first_supervisor = CodexHookIngressSupervisor::new();
    let second_supervisor = CodexHookIngressSupervisor::new();
    let first_endpoint = IngressEndpoint::new(FIRST_WORKER_HOME_DIRECTORY);
    let second_endpoint = IngressEndpoint::new(SECOND_WORKER_HOME_DIRECTORY);
    let first = first_endpoint
        .bind_server(&first_supervisor, &fixture)
        .await;
    let second = second_endpoint
        .bind_server(&second_supervisor, &fixture)
        .await;
    let first_runtime = first_endpoint.bound_runtime(&first, &fixture).await;
    let second_runtime = second_endpoint.bound_runtime(&second, &fixture).await;

    drop((first, first_runtime, first_supervisor));
    first_endpoint.finish();
    assert_active(&second_runtime);

    drop(second);
    let successor = second_endpoint
        .bind_server(&second_supervisor, &fixture)
        .await;
    let successor_runtime = second_endpoint.bound_runtime(&successor, &fixture).await;
    assert_superseded(&second_runtime);
    assert_active(&successor_runtime);
    drop((
        successor,
        successor_runtime,
        second_runtime,
        second_supervisor,
    ));
    second_endpoint.finish();
}

#[test]
fn worker_registry_has_no_process_global_owner() {
    let supervisor = CodexHookIngressSupervisor::new();
    let registry = supervisor.registry_probe();
    assert!(registry.is_alive());
    drop(supervisor);
    assert!(!registry.is_alive());
}

fn assert_active(runtime: &super::super::runtime::CodexHookRuntime) {
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        CodexAuthorityView::Active(_)
    ));
}

fn assert_superseded(runtime: &super::super::runtime::CodexHookRuntime) {
    assert_eq!(
        runtime.subscribe_authority().borrow().clone(),
        CodexAuthorityView::Superseded
    );
}
