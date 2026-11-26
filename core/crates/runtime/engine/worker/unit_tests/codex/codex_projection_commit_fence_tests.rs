use crate::worker::tests::*;
const PROJECTION_AUTHORITY_COMMIT: usize = 1;
const PROJECTION_BINDING_COMMIT: usize = 2;
const PROJECTION_COMMIT_INCREMENT: usize = 1;

#[test]
fn runtime_takeover_at_projection_authority_commit_preserves_empty_publication() {
    let fixture = Fixture::new();
    let predecessor_record = lock_recover(&fixture.state.record).clone();
    let successor_record = successor_runtime(&predecessor_record);
    let predecessor = projection_authority(&fixture, &predecessor_record);

    assert!(
        publish_with_runtime_takeover(
            &fixture,
            &predecessor_record,
            &successor_record,
            &predecessor,
            PROJECTION_AUTHORITY_COMMIT,
        )
        .is_err()
    );
    assert_projection_empty(&fixture);
}

#[test]
fn runtime_takeover_at_projection_binding_commit_never_installs_stale_binding() {
    let fixture = Fixture::new();
    let predecessor_record = lock_recover(&fixture.state.record).clone();
    let successor_record = successor_runtime(&predecessor_record);
    let predecessor = projection_authority(&fixture, &predecessor_record);

    assert!(
        publish_with_runtime_takeover(
            &fixture,
            &predecessor_record,
            &successor_record,
            &predecessor,
            PROJECTION_BINDING_COMMIT,
        )
        .is_err()
    );
    let store = projection_binding_store(&fixture);
    assert_eq!(
        store
            .codex_authority_store()
            .unwrap()
            .load_in_namespace(fixture.state.namespace.lease())
            .unwrap(),
        Some(predecessor)
    );
    assert!(
        store
            .read_raw_in_namespace(fixture.state.namespace.lease())
            .unwrap()
            .is_none()
    );
}

#[test]
fn successor_repairs_authority_committed_before_binding() {
    let fixture = Fixture::new();
    let predecessor_record = lock_recover(&fixture.state.record).clone();
    let successor_record = successor_runtime(&predecessor_record);
    let predecessor = confirmed_exit_projection_authority(&fixture, &predecessor_record);
    assert!(
        publish_with_runtime_takeover(
            &fixture,
            &predecessor_record,
            &successor_record,
            &predecessor,
            PROJECTION_BINDING_COMMIT,
        )
        .is_err()
    );
    let mut successor = confirmed_exit_projection_authority(&fixture, &successor_record);
    successor.version.route_revision = SUCCESSOR_ROUTE_REVISION;

    publish_as_projection(&fixture, &successor_record, &successor).unwrap();
    assert_projection_pair(&fixture, &successor);
}

fn confirmed_exit_projection_authority(
    fixture: &Fixture,
    record: &RuntimeRecord,
) -> crate::agent::codex::authority::CodexBindingAuthority {
    let mut authority = projection_authority(fixture, record);
    authority.binding.state = crate::agent::AgentState::Exited;
    authority.binding.process_pid = None;
    authority.binding.process_start_time = None;
    authority
        .with_verified_resume_identity(&record.boot_id)
        .unwrap()
}

fn publish_with_runtime_takeover(
    fixture: &Fixture,
    expected: &RuntimeRecord,
    successor: &RuntimeRecord,
    authority: &crate::agent::codex::authority::CodexBindingAuthority,
    takeover_at: usize,
) -> anyhow::Result<()> {
    let fence = agent_monitor_publication_fence::acquire(&fixture.state, expected)?;
    let store = projection_binding_store(fixture);
    let commits = std::cell::Cell::new(usize::default());
    store
        .codex_authority_store()?
        .publish_in_namespace_with_commit_fence(
            fixture.state.namespace.lease(),
            store.path(),
            authority,
            || {
                let commit = commits.get() + PROJECTION_COMMIT_INCREMENT;
                commits.set(commit);
                if commit == takeover_at {
                    atomic_write_json(
                        &fixture.state.paths.runtime_record(fixture.terminal_id()),
                        successor,
                    )?;
                }
                fence.verify()
            },
        )
}

pub(super) fn projection_binding_store(fixture: &Fixture) -> AgentBindingStore {
    AgentBindingStore::new(fixture.state.paths.agent_binding(fixture.terminal_id()))
}

pub(super) fn assert_projection_empty(fixture: &Fixture) {
    let store = projection_binding_store(fixture);
    assert!(
        store
            .codex_authority_store()
            .unwrap()
            .load_in_namespace(fixture.state.namespace.lease())
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .read_raw_in_namespace(fixture.state.namespace.lease())
            .unwrap()
            .is_none()
    );
}
