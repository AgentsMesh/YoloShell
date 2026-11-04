use super::*;

#[path = "../../test_support/admission.rs"]
mod support;
use support::Fixture;

#[test]
fn publication_retains_exact_durable_admission() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    let record = fixture.record();
    let publication = storage.publish(&record).unwrap();
    let authority =
        CodexAdmissionAuthority::from_publication(storage.clone(), publication).unwrap();

    authority.verify().unwrap();
    assert_eq!(authority.record(), &record);
    assert_eq!(authority.path(), storage.path());

    std::fs::remove_file(storage.path()).unwrap();
    let replacement = storage.publish(&record).unwrap();
    let replacement = CodexAdmissionAuthority::from_publication(storage, replacement).unwrap();
    assert!(!authority.exactly_matches(&replacement));
    assert!(authority.verify().is_err());
    replacement.verify().unwrap();
}

#[test]
fn recovery_is_missing_or_exact() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    assert!(
        CodexAdmissionAuthority::recover(storage.clone())
            .unwrap()
            .is_none()
    );

    let record = fixture.record();
    storage.publish(&record).unwrap();
    let recovered = CodexAdmissionAuthority::recover(storage).unwrap().unwrap();
    recovered.verify().unwrap();
    assert_eq!(recovered.record(), &record);
}
