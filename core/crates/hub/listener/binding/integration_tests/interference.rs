use super::{constants::*, fixture::*};
use std::os::unix::fs::PermissionsExt;

#[test]
fn same_bytes_replacement_and_aba_restoration_permanently_revoke_live_store() {
    let fixture = Fixture::new(false);
    let bindings = fixture.open(false).unwrap();
    let before = fixture.configuration.current().unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    let original = fixture.directory.path().join(SAVED);
    std::fs::rename(fixture.path(), &original).unwrap();
    std::fs::write(fixture.path(), bytes).unwrap();
    std::fs::set_permissions(
        fixture.path(),
        std::fs::Permissions::from_mode(PRIVATE_MODE),
    )
    .unwrap();
    assert!(bindings.verify(&before).is_err());
    std::fs::rename(fixture.path(), fixture.directory.path().join(REPLACEMENT)).unwrap();
    std::fs::rename(original, fixture.path()).unwrap();
    assert!(bindings.verify(&before).is_err());
}

#[test]
fn retained_state_root_replacement_never_publishes_into_the_new_namespace() {
    let fixture = Fixture::new(false);
    let mut bindings = fixture.open(false).unwrap();
    let before = fixture.configuration.current().unwrap();
    let next = fixture.configuration.planned(&fixture.change()).unwrap();
    let root = fixture.hub.directory().unwrap().path().to_path_buf();
    let path = fixture.path();
    std::fs::rename(&root, fixture.directory.path().join(SAVED)).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(DIRECTORY_MODE)).unwrap();
    assert!(bindings.prepare(&before, &next, None).is_err());
    assert!(!path.exists());
}

#[test]
fn in_place_record_corruption_is_not_overwritten_by_a_prepared_successor() {
    let fixture = Fixture::new(false);
    let mut bindings = fixture.open(false).unwrap();
    let before = fixture.configuration.current().unwrap();
    let next = fixture.configuration.planned(&fixture.change()).unwrap();
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.path()).unwrap()).unwrap();
    record[CURRENT_FIELD][CONFIGURATION_FIELD][CHANGE_FIELD] =
        uuid::Uuid::new_v4().to_string().into();
    let bytes = serde_json::to_vec(&record).unwrap();
    std::fs::write(fixture.path(), &bytes).unwrap();
    assert!(bindings.prepare(&before, &next, None).is_err());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
}
