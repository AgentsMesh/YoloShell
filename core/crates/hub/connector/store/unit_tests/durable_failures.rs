use super::support::*;
use crate::{constants::MAX_DIRECTORY_ENTRIES, record::Record};
use yolo_paths::directory::PrivateDirectory;

const INVALID_MEMBER_NAME: &str = "not-a-connector.json";
const NON_CANONICAL_MEMBER_NAME: &str = "550E8400-E29B-41D4-A716-446655440000.json";
const RESIDUE_PREFIX: &str = ".inventory-residue-";
const EMPTY_RESIDUE: &[u8] = b"";

#[test]
fn a_member_cannot_change_registration_request_or_be_transplanted_between_ids() {
    for transplant in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        let other = fixture.seed();
        store.insert(seed.clone(), State::revoked(&seed)).unwrap();
        store.insert(other.clone(), State::initial(&other)).unwrap();
        drop(store);
        let mut record = Record {
            seed: seed.clone(),
            state: State::revoked(&seed),
        };
        if transplant {
            record = Record {
                seed: other.clone(),
                state: State::initial(&other),
            };
        } else {
            record.seed.request_id = uuid::Uuid::new_v4();
        }
        fixture.write(&fixture.member(&seed), &record);
        let bytes = std::fs::read(fixture.member(&seed)).unwrap();
        assert!(fixture.open().is_err());
        assert_eq!(std::fs::read(fixture.member(&seed)).unwrap(), bytes);
        assert_eq!(
            fixture.read(&fixture.member(&other))[STATE_FIELD],
            serde_json::to_value(State::initial(&other)).unwrap()
        );
    }
}

#[test]
fn json_member_names_require_a_canonical_connector_identity() {
    for name in [INVALID_MEMBER_NAME, NON_CANONICAL_MEMBER_NAME] {
        let fixture = Fixture::new();
        let store = fixture.open().unwrap();
        let index = fixture.file_identity(&fixture.index());
        let path = fixture.path.join(MEMBERS_DIRECTORY).join(name);
        std::fs::write(&path, EMPTY_RESIDUE).unwrap();
        assert!(store.verify().is_err());
        assert!(store.entries().is_err());
        assert_eq!(fixture.file_identity(&fixture.index()), index);
        drop(store);
        assert!(fixture.open().is_err());
        assert!(path.exists());
    }
}

#[test]
fn recovery_never_recreates_or_adopts_a_replaced_member_namespace() {
    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    drop(store);
    let index = fixture.file_identity(&fixture.index());
    let members = fixture.path.join(MEMBERS_DIRECTORY);
    std::fs::rename(&members, fixture.sibling(MOVED_DIRECTORY)).unwrap();
    assert!(fixture.open().is_err());
    assert!(!members.exists());
    PrivateDirectory::ensure(&members).unwrap();
    assert!(fixture.open().is_err());
    assert_eq!(fixture.file_identity(&fixture.index()), index);
}

#[test]
fn a_missing_or_symlinked_index_cannot_forget_a_revoked_member() {
    for symlink in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        store.insert(seed.clone(), State::revoked(&seed)).unwrap();
        drop(store);
        let member = fixture.file_identity(&fixture.member(&seed));
        let original = fixture.sibling(MOVED_DIRECTORY);
        std::fs::rename(fixture.index(), &original).unwrap();
        let index_bytes = std::fs::read(&original).unwrap();
        if symlink {
            std::os::unix::fs::symlink(&original, fixture.index()).unwrap();
        }
        assert!(fixture.open().is_err());
        assert_eq!(fixture.file_identity(&fixture.member(&seed)), member);
        assert_eq!(std::fs::read(original).unwrap(), index_bytes);
        if !symlink {
            assert!(!fixture.index().exists());
        }
    }
}

#[test]
fn unbounded_non_member_residue_is_rejected_without_losing_registered_state() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    store.insert(seed.clone(), State::revoked(&seed)).unwrap();
    let member = fixture.file_identity(&fixture.member(&seed));
    for index in 0..MAX_DIRECTORY_ENTRIES {
        std::fs::write(
            fixture
                .path
                .join(MEMBERS_DIRECTORY)
                .join(format!("{RESIDUE_PREFIX}{index}")),
            EMPTY_RESIDUE,
        )
        .unwrap();
    }
    assert!(store.verify().is_err());
    assert!(store.get(seed.connector_id).is_err());
    assert_eq!(fixture.file_identity(&fixture.member(&seed)), member);
    drop(store);
    assert!(fixture.open().is_err());
}
