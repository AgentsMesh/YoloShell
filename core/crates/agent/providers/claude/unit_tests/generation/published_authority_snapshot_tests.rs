use super::durable_state_publication_tests::published_fixture;
use super::support::{Fixture, PROCESS_PID, PROCESS_START, session_id};
use crate::agent::claude::generation::{AuthorityProcessIdentity, GenerationBegin};
use crate::agent::{AgentBindingStore, AgentKind, DetectionContext};
use uuid::Uuid;

const SUCCESSOR_PROCESS_OFFSET: u32 = 100;
const MALFORMED_AUTHORITY: &[u8] = b"not-json";

#[test]
fn stable_committed_publication_returns_the_binding_without_mutation() {
    let fixture = published_fixture();
    let store = binding_store(&fixture);
    let before = durable_bytes(&fixture);
    let binding = store
        .load_published_authoritative_in_namespace(
            &fixture.lease,
            &context(&fixture),
            fixture.runtime(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        Some(binding),
        store.read_raw_in_namespace(&fixture.lease).unwrap()
    );
    assert_eq!(durable_bytes(&fixture), before);
}

#[test]
fn pending_publication_is_not_visible_and_is_not_recovered() {
    let fixture = published_fixture();
    let current = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    fixture
        .store
        .begin_binding_publication_in_namespace(&fixture.lease, current.authentication(), &current)
        .unwrap();
    let before = durable_bytes(&fixture);
    assert!(load(&fixture).unwrap().is_none());
    assert_eq!(durable_bytes(&fixture), before);
}

#[test]
fn active_invocation_switch_during_snapshot_is_pending_not_an_error() {
    let fixture = published_fixture();
    let store = binding_store(&fixture);
    let binding_before = std::fs::read(fixture.binding_path()).unwrap();
    let successor = successor_begin(&fixture);
    let mut activated = None;
    let loaded = store
        .load_published_authoritative_with_test_hook(
            &fixture.lease,
            &context(&fixture),
            fixture.runtime(),
            || {
                activated = Some(
                    fixture
                        .store
                        .begin_in_namespace(&fixture.lease, &successor)?,
                );
                Ok(())
            },
        )
        .unwrap();
    assert!(loaded.is_none());
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.lease).unwrap(),
        activated
    );
    assert_eq!(
        std::fs::read(fixture.binding_path()).unwrap(),
        binding_before
    );
}

#[test]
fn stable_authority_corruption_remains_an_error() {
    let fixture = published_fixture();
    std::fs::write(fixture.store.path(), MALFORMED_AUTHORITY).unwrap();
    assert!(load(&fixture).is_err());
}

fn load(fixture: &Fixture) -> anyhow::Result<Option<crate::agent::AgentBinding>> {
    binding_store(fixture).load_published_authoritative_in_namespace(
        &fixture.lease,
        &context(fixture),
        fixture.runtime(),
    )
}

fn binding_store(fixture: &Fixture) -> AgentBindingStore {
    AgentBindingStore::new(fixture.binding_path())
}

fn context(fixture: &Fixture) -> DetectionContext {
    DetectionContext::new(
        fixture.begin.terminal_id,
        fixture.begin.cwd.clone(),
        [(AgentKind::ClaudeCode, fixture.begin.data_home.clone())],
    )
}

fn durable_bytes(fixture: &Fixture) -> (Vec<u8>, Vec<u8>) {
    (
        std::fs::read(fixture.binding_path()).unwrap(),
        std::fs::read(fixture.store.path()).unwrap(),
    )
}

fn successor_begin(fixture: &Fixture) -> GenerationBegin {
    GenerationBegin {
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
        process: AuthorityProcessIdentity {
            pid: PROCESS_PID + SUCCESSOR_PROCESS_OFFSET,
            start_time: PROCESS_START + u64::from(SUCCESSOR_PROCESS_OFFSET),
        },
        provider_session_id: session_id(),
        ..fixture.begin.clone()
    }
}
