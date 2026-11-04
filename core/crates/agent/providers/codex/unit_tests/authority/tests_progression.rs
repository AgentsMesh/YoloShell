use super::*;

const AUTHORITY_REVISION_INCREMENT: u64 = 1;
const SEQUENCE_INCREMENT: u64 = 1;

#[test]
fn newer_authority_revision_accepts_an_unchanged_event_sequence() {
    let fixture = Fixture::new();
    let current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&current).unwrap();
    let mut next = current.clone();
    next.version.authority_revision += AUTHORITY_REVISION_INCREMENT;

    fixture.publish(&next).unwrap();
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(next)
    );
}

#[test]
fn event_sequence_cannot_override_a_newer_authority_revision() {
    let fixture = Fixture::new();
    let mut current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    current.version.authority_revision += AUTHORITY_REVISION_INCREMENT;
    fixture.publish(&current).unwrap();
    let mut stale = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    stale.version.sequence = current.version.sequence + SEQUENCE_INCREMENT;

    assert!(fixture.publish(&stale).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(current)
    );
}

#[test]
fn one_authority_revision_cannot_name_two_event_sequences() {
    let fixture = Fixture::new();
    let current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&current).unwrap();
    let mut inconsistent = current.clone();
    inconsistent.version.sequence += SEQUENCE_INCREMENT;

    assert!(fixture.publish(&inconsistent).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(current)
    );
}
