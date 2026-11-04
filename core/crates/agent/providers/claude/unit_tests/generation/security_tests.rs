use super::super::SessionStartSource;
use super::support::*;
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn every_authentication_identity_field_is_required() {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let session_id = fixture.begin.provider_session_id.clone();
    let event = fixture.start(&session_id, SessionStartSource::Fresh);
    let valid = fixture.authentication();
    let mut cases = Vec::new();
    let mut changed = valid;
    changed.terminal_id = Uuid::new_v4();
    cases.push(changed);
    changed = valid;
    changed.runtime_epoch = Uuid::new_v4();
    cases.push(changed);
    changed = valid;
    changed.invocation_id = Uuid::new_v4();
    cases.push(changed);
    changed = valid;
    changed.hook_token = Uuid::new_v4();
    cases.push(changed);
    changed = valid;
    changed.process.start_time += 1;
    cases.push(changed);
    for authentication in cases {
        assert!(
            fixture
                .store
                .apply_in_namespace(&fixture.lease, authentication, &event)
                .is_err()
        );
    }
}

#[test]
fn malformed_begin_and_tampered_sidecar_fail_closed() {
    let mut fixture = Fixture::new();
    fixture.begin.provider_session_id = Uuid::new_v4().simple().to_string();
    assert!(
        fixture
            .store
            .begin_in_namespace(&fixture.lease, &fixture.begin)
            .is_err()
    );
    fixture.begin.provider_session_id = session_id();
    fixture.begin.cwd = PathBuf::from("relative");
    assert!(
        fixture
            .store
            .begin_in_namespace(&fixture.lease, &fixture.begin)
            .is_err()
    );

    fixture.begin.cwd = fixture.begin.data_home.join("project");
    let mut authority = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    authority.process.start_time = 0;
    fixture
        .lease
        .write_json(fixture.store.path(), &authority)
        .unwrap();
    assert!(fixture.store.load_in_namespace(&fixture.lease).is_err());
}

#[cfg(unix)]
#[test]
fn authority_symlink_is_never_followed() {
    let fixture = Fixture::new();
    let authority = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let outside = fixture.store.path().with_extension("outside");
    crate::paths::atomic_write_json(&outside, &authority).unwrap();
    std::fs::remove_file(fixture.store.path()).unwrap();
    std::os::unix::fs::symlink(&outside, fixture.store.path()).unwrap();
    assert!(fixture.store.load_in_namespace(&fixture.lease).is_err());
    assert_eq!(
        std::fs::read_to_string(outside).unwrap(),
        serde_json::to_string_pretty(&authority).unwrap()
    );
}
