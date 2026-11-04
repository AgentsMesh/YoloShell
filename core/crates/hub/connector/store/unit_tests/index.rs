use super::support::*;
use crate::{
    MAX_CONNECTORS,
    record::{Index, Member, Record},
};
use yolo_hub_identity::ConnectorId;

const DUPLICATE_MEMBERS: usize = 2;
const SCHEMA_INCREMENT: u16 = 1;

#[test]
fn compact_ready_index_bounds_entries_and_rejects_duplicate_requests_or_ids() {
    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    let mut index = store.index.clone();
    for _ in 0..MAX_CONNECTORS {
        let seed = fixture.seed();
        index.members.push(Member::Ready {
            connector_id: seed.connector_id,
            request_id: seed.request_id,
            seed_digest: seed.digest().unwrap(),
        });
    }
    index
        .validate(store.directory, store.members_directory, fixture.identity)
        .unwrap();
    assert!(
        serde_json::to_vec_pretty(&index).unwrap().len() as u64
            <= yolo_paths::constants::MAX_DURABLE_JSON_BYTES
    );
    let mut overflow = index.clone();
    overflow
        .members
        .push(index.members.first().unwrap().clone());
    assert!(
        overflow
            .validate(store.directory, store.members_directory, fixture.identity)
            .is_err()
    );
    let mut duplicate = store.index.clone();
    let seed = fixture.seed();
    duplicate.members = vec![
        Member::Preparing {
            initial: Record {
                seed: seed.clone(),
                state: State::initial(&seed)
            }
        };
        DUPLICATE_MEMBERS
    ];
    assert!(
        duplicate
            .validate(store.directory, store.members_directory, fixture.identity)
            .is_err()
    );
    if let Member::Preparing { initial } = duplicate.members.last_mut().unwrap() {
        initial.seed.connector_id = ConnectorId::new();
    }
    assert!(
        duplicate
            .validate(store.directory, store.members_directory, fixture.identity)
            .is_err()
    );
}

#[test]
fn index_schema_peer_and_member_directory_identity_are_binding() {
    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    let mut index: Index<State> = store.index.clone();
    index.schema_version += SCHEMA_INCREMENT;
    assert!(
        index
            .validate(store.directory, store.members_directory, fixture.identity)
            .is_err()
    );
    assert!(
        store
            .index
            .validate(store.members_directory, store.directory, fixture.identity)
            .is_err()
    );
    let other = Fixture::new();
    assert!(
        store
            .index
            .validate(store.directory, store.members_directory, other.identity)
            .is_err()
    );
}
