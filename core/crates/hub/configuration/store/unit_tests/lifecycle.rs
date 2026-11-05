use super::fixtures::{constants::*, *};
use crate::{ConfigurationStore, constants::*};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_management_protocol::{CONFIGURATION_SCHEMA, FIRST_CONFIGURATION_REVISION};

#[test]
fn first_open_creates_one_identity_bound_revision_and_reopen_preserves_it() {
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    let state = store.current().unwrap();
    assert_eq!(state.identity, fixture.identity);
    assert_eq!(state.revision, FIRST_CONFIGURATION_REVISION);
    assert_eq!(state.schema_version, CONFIGURATION_SCHEMA);
    assert!(!state.last_change.is_nil());
    assert_eq!(state.configuration, Configuration::default());
    let before = fixture.bytes();
    assert!(fixture.open(None).is_err());
    drop(store);
    let reopened = fixture.open(Some(Configuration::default())).unwrap();
    assert_eq!(reopened.current().unwrap(), state);
    assert_eq!(fixture.bytes(), before);
}

#[test]
fn explicit_seed_is_persisted_but_conflicting_seed_never_overwrites_existing_configuration() {
    let fixture = Fixture::new();
    let store = fixture.open(Some(changed())).unwrap();
    assert_eq!(store.current().unwrap().configuration, changed());
    drop(store);
    let before = fixture.bytes();
    let error = fixture.open(Some(Configuration::default())).err().unwrap();
    assert_eq!(error.to_string(), SEED_CONFLICT);
    assert_eq!(fixture.bytes(), before);
    assert!(fixture.open(Some(changed())).is_ok());
}

#[test]
fn invalid_seed_does_not_create_record_and_wrong_identity_cannot_adopt_existing_record() {
    let fixture = Fixture::new();
    assert!(
        fixture
            .open(Some(invalid_configuration(fixture.identity)))
            .is_err()
    );
    assert!(!fixture.path.join(RECORD_FILE).exists());
    let store = fixture.open(None).unwrap();
    drop(store);
    let before = fixture.bytes();
    assert!(ConfigurationStore::open(fixture.directory(), other_identity(), None).is_err());
    assert_eq!(fixture.bytes(), before);
}

#[test]
fn copied_configuration_record_cannot_enroll_another_directory() {
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    let record = fixture.record();
    let clone =
        yolo_paths::directory::PrivateDirectory::ensure(&fixture.sibling(CLONE_DIR)).unwrap();
    let path = clone.path().join(RECORD_FILE);
    let namespace = yolo_paths::namespace_lease::NamespaceLease::from_directories(vec![(
        TEST_NAMESPACE,
        clone,
    )])
    .unwrap();
    namespace.write_json(&path, &record).unwrap();
    assert!(
        ConfigurationStore::open(
            namespace.directory(path.parent().unwrap()).unwrap(),
            fixture.identity,
            None
        )
        .is_err()
    );
    assert!(store.current().is_ok());
}
