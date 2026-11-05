use super::{constants::*, fixture::Fixture};
use crate::{constants::SCHEMA_VERSION, index::Index, record::Record};
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn index_and_origin_record_require_exact_schema_entry_directory_and_catalogue() {
    let fixture = Fixture::new();
    let root = PrivateDirectory::open(&fixture.path)
        .unwrap()
        .durable_identity()
        .unwrap();
    let entry = fixture.peer.entry.identity;
    let record = Record::new(root, entry, &fixture.peer.export.catalogue);
    assert_eq!(
        record.history(root, entry, routing_limits()).unwrap(),
        fixture.history()
    );
    let decoded: Record = serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(decoded, record);
    let mut index = Index {
        schema_version: SCHEMA_VERSION,
        directory: root,
        entry,
        origins: vec![fixture.peer.origin.identity.hub()],
    };
    index.validate(root, entry, routing_limits()).unwrap();
    let mut changed = record.clone();
    changed.schema_version = SCHEMA_VERSION + 1;
    assert!(changed.history(root, entry, routing_limits()).is_err());
    let other = Identity::new().identity;
    assert!(record.history(root, other, routing_limits()).is_err());
    assert!(index.validate(root, other, routing_limits()).is_err());
    let other_root = PrivateDirectory::open_or_create_strict(&fixture.sibling(CLONE))
        .unwrap()
        .durable_identity()
        .unwrap();
    assert!(record.history(other_root, entry, routing_limits()).is_err());
    assert!(index.validate(other_root, entry, routing_limits()).is_err());
    index.schema_version = SCHEMA_VERSION + 1;
    assert!(index.validate(root, entry, routing_limits()).is_err());
    let mut forged = record;
    forged.catalogue.payload.revision += STEP;
    assert!(forged.history(root, entry, routing_limits()).is_err());
}

#[test]
fn index_rejects_duplicates_local_origins_and_capacity_before_file_access() {
    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    let mut index = store.index.clone();
    let origin = fixture.peer.origin.identity.hub();
    index.origins = vec![origin, origin];
    assert!(
        index
            .validate(index.directory, index.entry, routing_limits())
            .is_err()
    );
    index.origins = vec![index.entry.hub()];
    assert!(
        index
            .validate(index.directory, index.entry, routing_limits())
            .is_err()
    );
    index.origins = vec![origin, Identity::new().identity.hub()];
    index.origins.sort();
    let mut limits = routing_limits();
    limits.max_known_agents = std::num::NonZeroUsize::new(ONE_ORIGIN).unwrap();
    assert!(
        index
            .validate(index.directory, index.entry, limits)
            .is_err()
    );
    index.origins.reverse();
    assert!(
        index
            .validate(index.directory, index.entry, routing_limits())
            .is_err()
    );
}
