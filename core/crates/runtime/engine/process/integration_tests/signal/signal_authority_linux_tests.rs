use super::*;

const RETAINED_MARKER: u8 = 1;
const AUTHORITY_MARKER: u8 = 2;

#[test]
fn changed_membership_retains_containment_and_signal_authority() {
    let mut containment = Some(RETAINED_MARKER);
    let mut authority = Some(AUTHORITY_MARKER);
    let preparation = LinuxContainmentPreparation::MembershipChanged;
    let mut removed = false;

    commit_preparation(&mut containment, &mut authority, &mut removed, &preparation);

    assert_eq!(containment, Some(RETAINED_MARKER));
    assert_eq!(authority, Some(AUTHORITY_MARKER));
    assert!(!removed);
}

#[test]
fn removed_containment_releases_force_authority() {
    let mut containment = Some(RETAINED_MARKER);
    let mut authority = Some(AUTHORITY_MARKER);
    let preparation = LinuxContainmentPreparation::Removed;
    let mut removed = false;

    commit_preparation(&mut containment, &mut authority, &mut removed, &preparation);

    assert!(containment.is_none());
    assert!(authority.is_none());
    assert!(removed);
}

#[test]
fn retained_preparation_keeps_authority_until_finalization() {
    let mut containment = Some(RETAINED_MARKER);
    let mut authority = Some(AUTHORITY_MARKER);
    let preparation = LinuxContainmentPreparation::retained_for_test();
    let mut removed = false;

    commit_preparation(&mut containment, &mut authority, &mut removed, &preparation);

    assert_eq!(containment, Some(RETAINED_MARKER));
    assert_eq!(authority, Some(AUTHORITY_MARKER));
    clear_authority(&mut containment, &mut authority);
    assert!(!removed);

    assert!(containment.is_none());
    assert!(authority.is_none());
}
