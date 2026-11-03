use super::support::*;
use crate::agent::hooks::{ApplyDisposition, EventKind, HookProviderKind, Reducer, ReducerSnapshot};
use crate::codex_hooks::{CodexOutboxState, CodexSnapshotCursor};
use std::collections::VecDeque;

const EXHAUSTION_EVENT_UUID_BASE: u128 = 0x81000000000000000000000000000000;

pub(super) fn install_exhausted_active_authority(fixture: &Fixture) {
    let authority = seeded_reducer(fixture).exhausted_active_snapshot_for_test();
    install_authority(fixture, authority);
}

pub(super) fn install_penultimate_active_authority(fixture: &Fixture) {
    let authority = seeded_reducer(fixture).penultimate_active_snapshot_for_test();
    install_authority(fixture, authority);
}

pub(super) fn restored_reducer(
    fixture: &Fixture,
    runtime: &super::super::runtime::CodexHookRuntime,
) -> Reducer {
    let (_, snapshot) = CodexSnapshotCursor::open(runtime.snapshot_path()).unwrap();
    Reducer::restore(
        snapshot.unwrap(),
        fixture.identity.clone(),
        HookProviderKind::Codex,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
    )
    .unwrap()
}

fn seeded_reducer(fixture: &Fixture) -> Reducer {
    let mut reducer = Reducer::new(
        fixture.identity.clone(),
        HookProviderKind::Codex,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
    )
    .unwrap();
    assert_eq!(
        reducer.apply(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap(),
        ApplyDisposition::Applied
    );
    for index in 0..crate::agent::hooks::MAX_REPLAY_WINDOW {
        let sequence = SECOND_EVENT_SEQUENCE
            .checked_add(u64::try_from(index).unwrap())
            .unwrap();
        let event_id = EXHAUSTION_EVENT_UUID_BASE
            .checked_add(u128::try_from(index).unwrap())
            .map(uuid::Uuid::from_u128)
            .unwrap();
        let event = envelope(
            fixture.identity.clone(),
            sequence,
            event_id,
            EventKind::UserPromptSubmit,
        );
        assert_eq!(reducer.apply(event).unwrap(), ApplyDisposition::Applied);
    }
    reducer
}

fn install_authority(
    fixture: &Fixture,
    authority: (ReducerSnapshot, Vec<crate::agent::hooks::HookEnvelope>),
) {
    let (snapshot, events) = authority;
    let last_sequence = events.last().unwrap().sequence.0;
    let outbox = CodexOutboxState::acknowledged_fixture_for_test(
        fixture.identity.clone(),
        last_sequence,
        last_sequence.checked_add(FIRST_EVENT_SEQUENCE).unwrap(),
        VecDeque::from(events),
    );
    crate::paths::atomic_write_json(&fixture.runtime.outbox_path(), &outbox).unwrap();
    let (cursor, installed) = CodexSnapshotCursor::open(fixture.runtime.snapshot_path()).unwrap();
    assert!(installed.is_some());
    cursor.persist(&snapshot).unwrap();
}
