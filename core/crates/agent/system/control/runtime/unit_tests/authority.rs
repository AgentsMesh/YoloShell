use super::{constants::*, fixture::*};
use crate::{ControlRuntime, FreshInitialization, JournalError, anchor::AnchorRecord};
use yolo_paths::directory::PrivateDirectory;

#[test]
fn unrelated_retained_session_and_nil_runtime_scope_are_rejected() {
    let fixture = Fixture::new();
    let directory =
        PrivateDirectory::ensure(&fixture.root.path().canonicalize().unwrap().join(SIBLING))
            .unwrap();
    assert!(matches!(
        FreshInitialization::capture(directory, &fixture.paths, &fixture.runtime),
        Err(JournalError::ScopeMismatch)
    ));
    let mut current = fixture.runtime.clone();
    current.runtime_epoch = uuid::Uuid::nil();
    assert!(matches!(
        ControlRuntime::open_existing(fixture.session(), &fixture.paths, &current),
        Err(JournalError::InvalidRequest)
    ));
}

#[test]
fn sidecar_schema_and_journal_scope_are_checked_without_exposing_runtime_secrets() {
    let fixture = Fixture::new();
    let owner = fixture.initialize();
    let mut record: AnchorRecord = fixture.read(&fixture.anchor());
    record.schema += STEP as u16;
    assert!(record.validate(&fixture.runtime).is_err());
    let other = Fixture::new();
    let other_owner = other.initialize();
    let other_anchor: AnchorRecord = other.read(&other.anchor());
    let mut foreign: AnchorRecord = fixture.read(&fixture.anchor());
    foreign.journal = other_anchor.journal;
    assert!(foreign.validate(&fixture.runtime).is_err());
    other_owner.verify(&other.runtime).unwrap();
    let mut changed = fixture.runtime.clone();
    changed.auth_token = WRONG_SCOPE.into();
    let error = owner.verify(&changed).unwrap_err();
    assert_eq!(error, JournalError::ScopeMismatch);
    assert!(!error.to_string().contains(AUTH_TOKEN));
    assert!(!error.to_string().contains(WRONG_SCOPE));
    owner.verify(&fixture.runtime).unwrap();
}
