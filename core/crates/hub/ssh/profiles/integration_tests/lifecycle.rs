use super::fixture::Fixture;
use yolo_hub_ssh_profile_protocol_test_support::{constants::*, *};
use yolo_hub_ssh_profiles::*;

#[test]
fn empty_registry_can_add_rotate_remove_and_reopen_profiles_with_exact_replay() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let empty = store.current().unwrap();
    assert!(empty.configuration.profiles.is_empty());
    assert!(store.resolve(&profile(0).profile).is_err());
    let added_request = change(&empty, configuration());
    let added = store.apply(added_request.clone()).unwrap();
    assert_eq!(added.revision, empty.revision + SSH_PROFILE_REVISION_STEP);
    assert_eq!(store.resolve(&profile(0).profile).unwrap(), profile(0));
    let bytes = fixture.bytes();
    assert_eq!(store.apply(added_request.clone()).unwrap(), added);
    assert_eq!(fixture.bytes(), bytes);
    drop(store);
    let mut store = fixture.open(None).unwrap();
    assert_eq!(store.current().unwrap(), added);
    assert_eq!(store.apply(added_request).unwrap(), added);
    let mut rotated = configuration();
    rotated.profiles[0].profile =
        SshProfileRef::new(profile(0).profile.profile_id(), REVISION_B).unwrap();
    let state = store.apply(change(&added, rotated.clone())).unwrap();
    assert!(store.resolve(&profile(0).profile).is_err());
    assert_eq!(
        store.resolve(&rotated.profiles[0].profile).unwrap(),
        rotated.profiles[0]
    );
    let cleared = store
        .apply(change(&state, SshProfileConfiguration::default()))
        .unwrap();
    assert!(cleared.configuration.profiles.is_empty());
    store.verify().unwrap();
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), cleared);
}

#[test]
fn conflicting_or_reused_requests_do_not_write_or_disable_subsequent_valid_changes() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let old = store.current().unwrap();
    let request = change(&old, configuration());
    let committed = store.apply(request.clone()).unwrap();
    let bytes = fixture.bytes();
    let stale = change(&old, SshProfileConfiguration::default());
    assert!(store.apply(stale).unwrap_err().is::<RevisionConflict>());
    let mut reused = request;
    reused.configuration = SshProfileConfiguration::default();
    assert!(store.apply(reused).unwrap_err().is::<RevisionConflict>());
    let mut invalid = change(&committed, configuration());
    invalid.request_id = uuid::Uuid::nil();
    assert!(store.apply(invalid).is_err());
    assert_eq!(fixture.bytes(), bytes);
    assert_eq!(store.current().unwrap(), committed);
    store
        .apply(change(&committed, SshProfileConfiguration::default()))
        .unwrap();
}

#[test]
fn seed_is_initial_only_and_readonly_resolution_preserves_registry_record_identity() {
    use std::os::unix::fs::MetadataExt;
    let fixture = Fixture::new();
    let store = fixture.open(Some(configuration())).unwrap();
    let before = std::fs::metadata(fixture.record()).unwrap();
    let bytes = fixture.bytes();
    assert_eq!(store.configuration().unwrap(), configuration());
    assert_eq!(store.resolve(&profile(0).profile).unwrap(), profile(0));
    store.verify().unwrap();
    let after = std::fs::metadata(fixture.record()).unwrap();
    assert_eq!(
        (
            before.dev(),
            before.ino(),
            before.len(),
            before.mode(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec()
        ),
        (
            after.dev(),
            after.ino(),
            after.len(),
            after.mode(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec()
        )
    );
    assert_eq!(fixture.bytes(), bytes);
    drop(store);
    assert!(
        fixture
            .open(Some(SshProfileConfiguration::default()))
            .is_err()
    );
    assert_eq!(fixture.bytes(), bytes);
    assert!(fixture.open(Some(configuration())).is_ok());
}
