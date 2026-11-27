use crate::worker::tests::*;
const PROJECTION_ROUTE_REVISION: u64 = 1;
pub(super) const SUCCESSOR_ROUTE_REVISION: u64 = PROJECTION_ROUTE_REVISION + 1;
const PROJECTION_EVENT_SEQUENCE: u64 = crate::agent::hooks::FIRST_SEQUENCE;
const PROJECTION_AUTHORITY_REVISION: u64 = 1;
const PROJECTION_INVOCATION_ID: u128 = 0x10000000000000000000000000000001;
const ALTERNATING_WRITER_ROUNDS: usize = 6;
const NEXT_WRITER_OFFSET: usize = 1;
const RUNTIME_TIME_INCREMENT: i64 = 1;

#[test]
fn predecessor_projection_cannot_fresh_cas_after_successor_publication() {
    let fixture = Fixture::new();
    let predecessor_record = lock_recover(&fixture.state.record).clone();
    let predecessor = projection_authority(&fixture, &predecessor_record);
    publish_as_projection(&fixture, &predecessor_record, &predecessor).unwrap();

    let successor_record = successor_runtime(&predecessor_record);
    atomic_write_json(
        &fixture.state.paths.runtime_record(fixture.terminal_id()),
        &successor_record,
    )
    .unwrap();
    let mut successor = projection_authority(&fixture, &successor_record);
    successor.version.route_revision = SUCCESSOR_ROUTE_REVISION;
    publish_as_projection(&fixture, &successor_record, &successor).unwrap();

    assert!(publish_as_projection(&fixture, &predecessor_record, &predecessor).is_err());
    assert_projection_pair(&fixture, &successor);
}

#[test]
fn alternating_runtime_writers_never_let_the_predecessor_reclaim_publication() {
    let fixture = Fixture::new();
    let first_record = lock_recover(&fixture.state.record).clone();
    let records = [first_record.clone(), successor_runtime(&first_record)];
    let mut authorities: Vec<_> = records
        .iter()
        .map(|record| projection_authority(&fixture, record))
        .collect();
    let runtime_path = fixture.state.paths.runtime_record(fixture.terminal_id());

    for round in usize::default()..ALTERNATING_WRITER_ROUNDS {
        let current = round % records.len();
        let predecessor = (current + NEXT_WRITER_OFFSET) % records.len();
        authorities[current].version.route_revision = PROJECTION_ROUTE_REVISION + round as u64;
        atomic_write_json(&runtime_path, &records[current]).unwrap();
        publish_as_projection(&fixture, &records[current], &authorities[current]).unwrap();

        assert!(
            publish_as_projection(&fixture, &records[predecessor], &authorities[predecessor])
                .is_err()
        );
        assert_projection_pair(&fixture, &authorities[current]);
    }
}

#[test]
fn successor_runtime_cannot_publish_below_the_durable_route_floor() {
    let fixture = Fixture::new();
    let predecessor_record = lock_recover(&fixture.state.record).clone();
    let mut current = projection_authority(&fixture, &predecessor_record);
    current.version.route_revision = SUCCESSOR_ROUTE_REVISION;
    publish_as_projection(&fixture, &predecessor_record, &current).unwrap();

    let successor_record = successor_runtime(&predecessor_record);
    atomic_write_json(
        &fixture.state.paths.runtime_record(fixture.terminal_id()),
        &successor_record,
    )
    .unwrap();
    let stale = projection_authority(&fixture, &successor_record);

    assert!(publish_as_projection(&fixture, &successor_record, &stale).is_err());
    assert_projection_pair(&fixture, &current);
}

pub(super) fn publish_as_projection(
    fixture: &Fixture,
    expected: &RuntimeRecord,
    authority: &crate::agent::codex::authority::CodexBindingAuthority,
) -> anyhow::Result<()> {
    publish_projection_for_state(&fixture.state, expected, authority)
}

pub(super) fn publish_projection_for_state(
    state: &WorkerState,
    expected: &RuntimeRecord,
    authority: &crate::agent::codex::authority::CodexBindingAuthority,
) -> anyhow::Result<()> {
    let fence = agent_monitor_publication_fence::acquire(state, expected)?;
    let store = AgentBindingStore::new(state.paths.agent_binding(state.terminal_id()));
    store
        .codex_authority_store()?
        .publish_in_namespace_with_commit_fence(
            state.namespace.lease(),
            store.path(),
            authority,
            || fence.verify(),
        )
}

pub(super) fn projection_authority(
    fixture: &Fixture,
    record: &RuntimeRecord,
) -> crate::agent::codex::authority::CodexBindingAuthority {
    let projected = binding(fixture, crate::agent::AgentState::Running);
    let provider = crate::agent::hooks::ProcessIdentity::new(
        projected.process_pid.unwrap(),
        projected.process_start_time.unwrap(),
    );
    crate::agent::codex::authority::CodexBindingAuthority::new(
        projection_runtime(record),
        provider,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        crate::agent::codex::authority::CodexAuthorityVersion {
            route_revision: PROJECTION_ROUTE_REVISION,
            invocation_id: Uuid::from_u128(PROJECTION_INVOCATION_ID),
            generation: crate::agent::hooks::Generation::INITIAL,
            authority_revision: PROJECTION_AUTHORITY_REVISION,
            sequence: PROJECTION_EVENT_SEQUENCE,
        },
        crate::agent::codex::authority::CodexAuthorityStatus::Active,
        projected,
    )
    .unwrap()
}

fn projection_runtime(
    record: &RuntimeRecord,
) -> crate::agent::authority::AuthorityRuntimeIdentity {
    crate::agent::authority::AuthorityRuntimeIdentity {
        runtime_epoch: record.runtime_epoch,
        worker: crate::agent::authority::AuthorityProcessIdentity {
            pid: record.worker_pid,
            start_time: record.worker_start_time,
        },
    }
}

pub(super) fn successor_runtime(predecessor: &RuntimeRecord) -> RuntimeRecord {
    let mut successor = predecessor.clone();
    successor.runtime_epoch = Uuid::new_v4();
    successor.worker_id = Uuid::new_v4();
    successor.started_at_ms += RUNTIME_TIME_INCREMENT;
    successor
}

pub(super) fn assert_projection_pair(
    fixture: &Fixture,
    expected: &crate::agent::codex::authority::CodexBindingAuthority,
) {
    let store = AgentBindingStore::new(fixture.state.paths.agent_binding(fixture.terminal_id()));
    assert_eq!(
        store
            .codex_authority_store()
            .unwrap()
            .load_in_namespace(fixture.state.namespace.lease())
            .unwrap()
            .as_ref(),
        Some(expected)
    );
    assert_eq!(
        store
            .read_raw_in_namespace(fixture.state.namespace.lease())
            .unwrap()
            .as_ref(),
        Some(&expected.binding)
    );
}
