use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::server_binding_repair_fixture::confirmed_exit_authority;
use super::support::Fixture;
use crate::agent::{AgentBinding, AgentBindingStore, AgentState};
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts,
    CodexHookIntake, CodexRetirementState,
};
use crate::process::{ProcessIdentity, ProcessIdentityObservation};

const HOME_DIRECTORY: &str = "server-binding-repair-home";
const INITIAL_ROUTE_REVISION: u64 = 1;
const STALE_START_TIME_INCREMENT: u64 = 1;
const NO_ROUTES: usize = 0;
const SINGLE_ROUTE: usize = 1;

pub(super) enum BindingDamage {
    Running,
    Missing,
}

pub(super) async fn assert_worker_preserves_binding_damage(damage: BindingDamage) {
    let fixture = gone_fixture();
    assert_eq!(fixture.process.observe(), ProcessIdentityObservation::Gone);
    let endpoint = IngressEndpoint::new(HOME_DIRECTORY);
    let admission = persist_admission(&endpoint, &fixture);
    let authority = confirmed_exit_authority(&endpoint, &fixture, &admission);
    let store = AgentBindingStore::new(endpoint.paths.agent_binding(Fixture::terminal_id()));
    publish_authority(&endpoint, &store, &authority);
    let expected = damage_binding(
        &endpoint,
        &store,
        &authority,
        fixture.identity.process,
        damage,
    );

    let supervisor = CodexHookIngressSupervisor::new();
    let server = endpoint.bind_server(&supervisor, &fixture).await;

    assert_eq!(server.router().len().unwrap(), NO_ROUTES);
    assert!(matches!(
        retirement_state(&admission),
        CodexRetirementState::Retired(_)
    ));
    assert_eq!(
        store
            .read_raw_in_namespace(endpoint.namespace.lease())
            .unwrap(),
        expected
    );
    assert_authority(&endpoint, &store, &authority);

    drop((server, supervisor));
    endpoint.finish();
}

pub(super) async fn assert_revoked_authority_stays_bindingless() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(HOME_DIRECTORY);
    let admission = persist_admission(&endpoint, &fixture);
    let authority = confirmed_exit_authority(&endpoint, &fixture, &admission);
    let store = AgentBindingStore::new(endpoint.paths.agent_binding(Fixture::terminal_id()));
    publish_authority(&endpoint, &store, &authority);
    let authority_store = store.codex_authority_store().unwrap();
    let (_, identity) = endpoint
        .namespace
        .lease()
        .read_json_with_identity::<crate::agent::codex::authority::CodexBindingAuthority>(
            authority_store.path(),
        )
        .unwrap();
    authority_store
        .revoke_exact_in_namespace_with_commit_fence(
            endpoint.namespace.lease(),
            &authority,
            identity,
            || Ok(()),
        )
        .unwrap();
    endpoint.namespace.remove_file(store.path()).unwrap();

    let supervisor = CodexHookIngressSupervisor::new();
    let server = endpoint.bind_server(&supervisor, &fixture).await;

    assert_eq!(server.router().len().unwrap(), SINGLE_ROUTE);
    assert_eq!(
        store
            .read_raw_in_namespace(endpoint.namespace.lease())
            .unwrap(),
        None
    );
    assert_authority(&endpoint, &store, &authority.revoked_for_test().unwrap());
    drop((server, supervisor));
    endpoint.finish();
}

fn gone_fixture() -> Fixture {
    let current = ProcessIdentity::capture(std::process::id()).unwrap();
    let gone = ProcessIdentity {
        pid: current.pid,
        start_time: current
            .start_time
            .checked_add(STALE_START_TIME_INCREMENT)
            .unwrap(),
    };
    let mut fixture = Fixture::new();
    fixture.process = gone;
    fixture.identity.process = crate::agent::hooks::ProcessIdentity::new(gone.pid, gone.start_time);
    fixture
}

fn persist_admission(endpoint: &IngressEndpoint, fixture: &Fixture) -> CodexAdmissionAuthority {
    let request = endpoint.admission_context(fixture);
    let record =
        CodexAdmissionRecord::capture_with_route_revision(&request, INITIAL_ROUTE_REVISION)
            .unwrap();
    let artifacts = CodexHookArtifacts::new(
        &endpoint.paths,
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        fixture.identity.invocation_id,
    )
    .unwrap();
    let storage = CodexAdmissionStorage::for_identity(&artifacts, &fixture.identity).unwrap();
    let publication = storage.publish_for_test(&record).unwrap();
    CodexHookIntake::for_identity(artifacts.intake_directory(), &fixture.identity).unwrap();
    CodexAdmissionAuthority::from_publication(storage, publication).unwrap()
}

fn publish_authority(
    endpoint: &IngressEndpoint,
    store: &AgentBindingStore,
    authority: &crate::agent::codex::authority::CodexBindingAuthority,
) {
    store
        .codex_authority_store()
        .unwrap()
        .publish_in_namespace_with_commit_fence(
            endpoint.namespace.lease(),
            store.path(),
            authority,
            || Ok(()),
        )
        .unwrap();
}

fn damage_binding(
    endpoint: &IngressEndpoint,
    store: &AgentBindingStore,
    authority: &crate::agent::codex::authority::CodexBindingAuthority,
    provider: crate::agent::hooks::ProcessIdentity,
    damage: BindingDamage,
) -> Option<AgentBinding> {
    match damage {
        BindingDamage::Running => {
            let mut running = authority.binding.clone();
            running.state = AgentState::Running;
            running.process_pid = Some(provider.pid);
            running.process_start_time = Some(provider.start_time);
            endpoint
                .namespace
                .write_json(store.path(), &running)
                .unwrap();
            Some(running)
        }
        BindingDamage::Missing => {
            endpoint.namespace.remove_file(store.path()).unwrap();
            None
        }
    }
}

fn retirement_state(admission: &CodexAdmissionAuthority) -> CodexRetirementState {
    admission
        .acquire_retirement_writer_fence()
        .unwrap()
        .read_state(admission)
        .unwrap()
}

fn assert_authority(
    endpoint: &IngressEndpoint,
    store: &AgentBindingStore,
    expected: &crate::agent::codex::authority::CodexBindingAuthority,
) {
    assert_eq!(
        store
            .codex_authority_store()
            .unwrap()
            .load_in_namespace(endpoint.namespace.lease())
            .unwrap(),
        Some(expected.clone())
    );
}
