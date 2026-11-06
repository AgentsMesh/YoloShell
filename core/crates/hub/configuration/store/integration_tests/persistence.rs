use crate::{constants::*, records, support::Fixture};
use yolo_hub_configuration_store::{ConfigurationStore, RevisionConflict};
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{CONFIGURATION_SCHEMA, FIRST_CONFIGURATION_REVISION};

#[test]
fn default_and_seeded_state_reopen_without_changing_identity_or_revision() {
    for seed in [None, Some(records::configuration())] {
        let fixture = Fixture::new();
        let store = fixture.open(seed.clone());
        let initial = store.current().unwrap();
        assert_eq!(initial.schema_version, CONFIGURATION_SCHEMA);
        assert_eq!(initial.identity, fixture.hub.peer_identity());
        assert_eq!(initial.revision, FIRST_CONFIGURATION_REVISION);
        assert!(!initial.last_change.is_nil());
        assert_eq!(initial.configuration, seed.clone().unwrap_or_default());
        let (_, identity) = fixture
            .namespace()
            .read_json_with_identity::<serde_json::Value>(&fixture.path.join(RECORD_FILE))
            .unwrap();
        drop(store);
        let reopened = fixture.open(seed);
        assert_eq!(reopened.current().unwrap(), initial);
        let (_, after) = fixture
            .namespace()
            .read_json_with_identity::<serde_json::Value>(&fixture.path.join(RECORD_FILE))
            .unwrap();
        assert_eq!(identity, after);
        drop(reopened);
        assert_eq!(fixture.open(None).current().unwrap(), initial);
    }
}

#[test]
fn mismatched_seed_and_invalid_configuration_never_replace_existing_state() {
    let fixture = Fixture::new();
    let seed = records::configuration();
    let store = fixture.open(Some(seed.clone()));
    let before = store.current().unwrap();
    drop(store);
    assert!(
        ConfigurationStore::open(
            fixture.directory(),
            fixture.hub.peer_identity(),
            Some(Configuration::default())
        )
        .is_err()
    );
    assert_eq!(fixture.open(None).current().unwrap(), before);
    let fixture = Fixture::new();
    let invalid = Configuration {
        trusted_origins: vec![fixture.hub.peer_identity()],
        file_roots: vec![],
        ..Configuration::default()
    };
    assert!(
        ConfigurationStore::open(
            fixture.directory(),
            fixture.hub.peer_identity(),
            Some(invalid)
        )
        .is_err()
    );
    assert!(!fixture.path.join(RECORD_FILE).exists());
}

#[test]
fn identity_checks_include_public_key_and_singleton_is_not_just_a_file_marker() {
    let fixture = Fixture::new();
    let store = fixture.open(None);
    assert!(
        ConfigurationStore::open(fixture.directory(), fixture.hub.peer_identity(), None).is_err()
    );
    let before = store.current().unwrap();
    drop(store);
    let local = fixture.hub.peer_identity();
    let wrong_hub = PeerIdentity::new(HubId::new(), *local.key()).unwrap();
    let wrong_key =
        PeerIdentity::new(local.hub(), [OTHER_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap();
    for wrong in [wrong_hub, wrong_key, records::identity()] {
        assert!(ConfigurationStore::open(fixture.directory(), wrong, None).is_err());
    }
    assert_eq!(fixture.open(None).current().unwrap(), before);
}

#[test]
fn invalid_schema_revision_request_identity_and_unknown_fields_fail_closed_on_reopen() {
    let mutations: &[fn(&mut serde_json::Value)] = &[
        |r| r[STATE_FIELD][SCHEMA_FIELD] = u16::MAX.into(),
        |r| r[STATE_FIELD][REVISION_FIELD] = u64::MIN.into(),
        |r| r[STATE_FIELD][LAST_CHANGE_FIELD] = uuid::Uuid::nil().to_string().into(),
        |r| r[INJECTED_FIELD] = true.into(),
        |r| r[STATE_FIELD][INJECTED_FIELD] = true.into(),
        |r| r[STATE_FIELD][CONFIGURATION_FIELD][INJECTED_FIELD] = true.into(),
        |r| r[DIRECTORY_FIELD] = serde_json::to_value(records::directory_identity()).unwrap(),
        |r| r[STATE_FIELD][IDENTITY_FIELD] = serde_json::to_value(records::identity()).unwrap(),
    ];
    for mutate in mutations {
        let fixture = Fixture::new();
        drop(fixture.open(None));
        let mut record = fixture.read();
        mutate(&mut record);
        fixture.write(&record);
        assert!(
            ConfigurationStore::open(fixture.directory(), fixture.hub.peer_identity(), None)
                .is_err()
        );
        assert_eq!(fixture.read(), record);
    }
    let fixture = Fixture::new();
    let store = fixture.open(None);
    drop(store);
    std::fs::write(fixture.path.join(RECORD_FILE), INVALID_DOCUMENT).unwrap();
    assert!(
        ConfigurationStore::open(fixture.directory(), fixture.hub.peer_identity(), None).is_err()
    );
    assert_eq!(
        std::fs::read(fixture.path.join(RECORD_FILE)).unwrap(),
        INVALID_DOCUMENT
    );
    let unrelated = records::state(records::identity());
    assert!(unrelated.validate().is_ok());
    assert!(
        anyhow::Error::new(RevisionConflict)
            .downcast_ref::<RevisionConflict>()
            .is_some()
    );
}
