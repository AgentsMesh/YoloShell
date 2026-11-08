use super::{constants::*, fixture::*, model::*};
use std::os::unix::fs::PermissionsExt;
use yolo_hub_client_access_protocol::{ACCESS_SCHEMA_VERSION, FIRST_ACCESS_REVISION};
use yolo_hub_client_access_store::ClientAccessStore;

#[test]
fn default_initialization_is_private_empty_and_exclusively_owned() {
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    let current = store.current().unwrap();
    assert_eq!(current.schema_version, ACCESS_SCHEMA_VERSION);
    assert_eq!(current.entry_hub, hub());
    assert_eq!(current.revision, FIRST_ACCESS_REVISION);
    assert!(current.clients.is_empty());
    assert_eq!(
        std::fs::metadata(fixture.record())
            .unwrap()
            .permissions()
            .mode()
            & MODE_MASK,
        PRIVATE_FILE_MODE
    );
    assert!(fixture.open(None).is_err());
    assert_eq!(store.current().unwrap(), current);
}

#[test]
fn exact_seed_and_default_reopen_never_rewrite_a_committed_record() {
    let fixture = Fixture::new();
    let expected = configuration();
    let store = fixture.open(Some(expected.clone())).unwrap();
    assert_eq!(store.current().unwrap(), expected);
    let bytes = fixture.bytes();
    let identity = fixture.file_identity();
    drop(store);
    for seed in [None, Some(expected.clone())] {
        let reopened = fixture.open(seed).unwrap();
        assert_eq!(reopened.current().unwrap(), expected);
        assert_eq!(fixture.bytes(), bytes);
        assert_eq!(fixture.file_identity(), identity);
    }
}

#[test]
fn conflicting_seed_cannot_overwrite_any_part_of_existing_configuration() {
    let mutations: &[fn(&mut yolo_hub_client_access_protocol::AccessConfiguration)] = &[
        |value| value.revision += REVISION_STEP,
        |value| value.clients.clear(),
        |value| value.clients[FIRST_CLIENT].expires_at_ms += TIME_STEP_MS,
    ];
    let fixture = Fixture::new();
    let original = configuration();
    drop(fixture.open(Some(original.clone())).unwrap());
    let bytes = fixture.bytes();
    let identity = fixture.file_identity();
    for change in mutations {
        let mut seed = original.clone();
        change(&mut seed);
        assert!(fixture.open(Some(seed)).is_err());
        assert_eq!(fixture.bytes(), bytes);
        assert_eq!(fixture.file_identity(), identity);
    }
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
}

#[test]
fn invalid_or_foreign_seed_creates_no_access_record() {
    let mutations: &[fn(&mut yolo_hub_client_access_protocol::AccessConfiguration)] = &[
        |value| value.schema_version = INVALID_SCHEMA,
        |value| value.entry_hub = other_hub(),
        |value| value.clients[FIRST_CLIENT].identity.client_id = uuid::Uuid::nil(),
        |value| value.clients.push(value.clients[FIRST_CLIENT].clone()),
    ];
    for change in mutations {
        let fixture = Fixture::new();
        let mut seed = configuration();
        change(&mut seed);
        assert!(fixture.open(Some(seed)).is_err());
        assert!(!fixture.record().exists());
        assert!(
            fixture
                .open(None)
                .unwrap()
                .current()
                .unwrap()
                .clients
                .is_empty()
        );
    }
}

#[test]
fn another_entry_hub_cannot_adopt_a_valid_record() {
    let fixture = Fixture::new();
    let original = configuration();
    drop(fixture.open(Some(original.clone())).unwrap());
    let bytes = fixture.bytes();
    assert!(ClientAccessStore::open(fixture.directory(), other_hub(), None).is_err());
    assert_eq!(fixture.bytes(), bytes);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
}
