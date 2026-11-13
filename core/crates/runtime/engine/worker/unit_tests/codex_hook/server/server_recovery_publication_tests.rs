use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::Fixture;
use super::runtime_retirement_intake_test_support::replace_same_contents;
use crate::worker::codex_hook::ingress_supervisor::CodexHookIngressSupervisor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const RECOVERY_ADMISSION_REPLACEMENT: &str = "socket-recovery-admission-replacement";
const POST_RECOVERY_ADMISSION_REPLACEMENT: &str = "socket-post-recovery-admission-replacement";
const POST_PUBLICATION_ADMISSION_REPLACEMENT: &str =
    "socket-post-publication-admission-replacement";
const RECOVERY_FAILURE_HOME: &str = "socket-recovery-failure-home";
const RECOVERY_ORDER_HOME: &str = "socket-recovery-order-home";

#[tokio::test]
async fn recovery_failure_never_publishes_the_hook_socket() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RECOVERY_FAILURE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    create_durable_route(&endpoint, &supervisor, &fixture).await;
    let socket = endpoint.socket_path();
    let admission = endpoint.admission_path(&fixture);
    crate::codex_hooks::set_admission_scan_validation_hook(move || {
        replace_same_contents(&admission, RECOVERY_ADMISSION_REPLACEMENT);
    });

    let result = endpoint.try_bind_server(&supervisor, &fixture).await;

    assert!(result.is_err());
    assert!(!socket.exists());
    drop(supervisor);
    endpoint.finish();
}

#[tokio::test]
async fn hook_socket_becomes_visible_only_after_recovery() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RECOVERY_ORDER_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    create_durable_route(&endpoint, &supervisor, &fixture).await;
    let socket = endpoint.socket_path();
    let observed = Arc::new(AtomicBool::new(false));
    let hook_observed = Arc::clone(&observed);
    let hook_socket = socket.clone();
    super::super::recovery_test_hook::set(move || {
        assert!(!hook_socket.exists());
        hook_observed.store(true, Ordering::SeqCst);
    });

    let server = endpoint.bind_server(&supervisor, &fixture).await;

    assert!(observed.load(Ordering::SeqCst));
    assert!(socket.exists());
    drop((server, supervisor));
    endpoint.finish();
}

#[tokio::test]
async fn post_recovery_change_withdraws_the_published_socket() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RECOVERY_ORDER_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    create_durable_route(&endpoint, &supervisor, &fixture).await;
    let socket = endpoint.socket_path();
    let hook_socket = socket.clone();
    let admission = endpoint.admission_path(&fixture);
    super::super::recovery_test_hook::set_before_socket_publication(move || {
        assert!(!hook_socket.exists());
        replace_same_contents(&admission, POST_RECOVERY_ADMISSION_REPLACEMENT);
    });

    let result = endpoint.try_bind_server(&supervisor, &fixture).await;

    assert!(result.is_err());
    assert!(!socket.exists());
    drop(supervisor);
    endpoint.finish();
}

#[tokio::test]
async fn post_publication_change_withdraws_the_published_socket() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RECOVERY_ORDER_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    create_durable_route(&endpoint, &supervisor, &fixture).await;
    let socket = endpoint.socket_path();
    let hook_socket = socket.clone();
    let admission = endpoint.admission_path(&fixture);
    super::super::recovery_test_hook::set_after_socket_publication(move || {
        assert!(hook_socket.exists());
        replace_same_contents(&admission, POST_PUBLICATION_ADMISSION_REPLACEMENT);
    });

    let result = endpoint.try_bind_server(&supervisor, &fixture).await;

    assert!(result.is_err());
    assert!(!socket.exists());
    drop(supervisor);
    endpoint.finish();
}

async fn create_durable_route(
    endpoint: &IngressEndpoint,
    supervisor: &CodexHookIngressSupervisor,
    fixture: &Fixture,
) {
    let server = endpoint.bind_server(supervisor, fixture).await;
    let runtime = endpoint.bound_runtime(&server, fixture).await;
    drop((server, runtime));
    assert!(!endpoint.socket_path().exists());
}
