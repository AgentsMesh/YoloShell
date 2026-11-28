use super::super::authority_registry::CodexAuthorityRegistry;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::Fixture;
use super::server_binding_repair_fixture::write_evidence;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::{
    CodexAuthorityStatus, CodexAuthorityVersion, CodexBindingAuthority,
};
use crate::agent::{
    AgentBinding, AgentBindingStore, AgentKind, AgentRecoveryAvailability, AgentState,
};
use uuid::Uuid;

const ROUTE_FLOOR: u64 = 7;
const PREDECESSOR_ROUTE_FLOOR: u64 = ROUTE_FLOOR - 1;
const NEXT_ROUTE_REVISION: u64 = ROUTE_FLOOR + 1;
const EXPECTED_ROUTE_COUNT: usize = 1;
const AUTHORITY_REVISION: u64 = 3;
const EVENT_SEQUENCE: u64 = 2;
const WORKER_PID: u32 = 91;
const WORKER_START_TIME: u64 = 92;
const DETECTED_AT_MS: i64 = 100;
const UPDATED_AT_MS: i64 = 200;
const HOME_DIRECTORY: &str = "durable-route-floor-home";

#[tokio::test]
async fn new_worker_allocates_above_the_durable_authority_route_floor() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(HOME_DIRECTORY);
    persist_confirmed_exit(&endpoint, &fixture);
    let supervisor = super::super::ingress_supervisor::CodexHookIngressSupervisor::new();
    let server = endpoint.bind_server(&supervisor, &fixture).await;
    let runtime = endpoint.bound_runtime(&server, &fixture).await;
    let routes = server.router().subscribe_committed().unwrap().snapshot;

    assert_eq!(routes.len(), EXPECTED_ROUTE_COUNT);
    assert_eq!(routes[0].revision, NEXT_ROUTE_REVISION);
    drop((runtime, server, supervisor));
    endpoint.finish();
}

fn persist_confirmed_exit(endpoint: &IngressEndpoint, fixture: &Fixture) {
    let binding_path = endpoint.paths.agent_binding(Fixture::terminal_id());
    let request = endpoint.admission_context(fixture);
    let provider_session_id = Uuid::new_v4().hyphenated().to_string();
    let evidence_path = write_evidence(&request, &provider_session_id);
    let binding = AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Fixture::terminal_id(),
        kind: AgentKind::Codex,
        provider_session_id,
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        cwd: request.cwd.display().to_string(),
        evidence_path: evidence_path.display().to_string(),
        data_home: request.data_home.display().to_string(),
        process_pid: None,
        process_start_time: None,
        state: AgentState::Exited,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    };
    let authority = CodexBindingAuthority::new_with_verified_resume_identity(
        AuthorityRuntimeIdentity {
            runtime_epoch: Uuid::new_v4(),
            worker: AuthorityProcessIdentity {
                pid: WORKER_PID,
                start_time: WORKER_START_TIME,
            },
        },
        fixture.identity.process,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        CodexAuthorityVersion {
            route_revision: ROUTE_FLOOR,
            invocation_id: Uuid::new_v4(),
            generation: fixture.identity.generation,
            authority_revision: AUTHORITY_REVISION,
            sequence: EVENT_SEQUENCE,
        },
        CodexAuthorityStatus::Active,
        binding,
        &crate::process::boot_id(),
    )
    .unwrap();
    AgentBindingStore::new(binding_path)
        .codex_authority_store()
        .unwrap()
        .publish_in_namespace_with_commit_fence(
            endpoint.namespace.lease(),
            &endpoint.paths.agent_binding(Fixture::terminal_id()),
            &authority,
            || Ok(()),
        )
        .unwrap();
}

#[test]
fn registry_clock_observes_the_largest_durable_floor() {
    let registry = CodexAuthorityRegistry::default();
    registry.observe_route_publication_floor(ROUTE_FLOOR);
    registry.observe_route_publication_floor(PREDECESSOR_ROUTE_FLOOR);

    assert_eq!(
        registry
            .route_publication_clock()
            .load(std::sync::atomic::Ordering::Acquire),
        ROUTE_FLOOR
    );
}
