use crate::process::ProcessIdentityRelation;
#[cfg(target_os = "macos")]
use crate::pty_custody::CustodyDurableState;

#[path = "../../test_support/replacement_test_support.rs"]
mod support;
use support::Fixture;

#[path = "replacement_validation_matrix_tests.rs"]
mod matrix_tests;

#[cfg(target_os = "macos")]
#[test]
fn proven_gone_guardian_installs_an_exact_successor() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    fixture.validate(&fixture.bootstrap).unwrap();

    let preparation = fixture.install().unwrap();
    let (prepared, identity) = fixture.current();
    assert_eq!(identity, preparation.custody_file_identity());
    let CustodyDurableState::PreparedGuardian {
        predecessor,
        candidate,
    } = &prepared.state
    else {
        unreachable!();
    };
    assert_eq!(predecessor.as_ref(), &fixture.snapshot.record().state);
    assert!(fixture.binding().matches_lease_identity(candidate));
    let armed = prepared
        .arm_prepared_guardian(&fixture.request, &fixture.offer, fixture.binding())
        .unwrap();
    assert!(armed.matches_candidate(&fixture.offer, *candidate));
}

#[test]
fn indeterminate_guardian_cannot_mutate_the_predecessor() {
    let fixture = Fixture::new(ProcessIdentityRelation::Unknown);
    let before = fixture.current();

    assert!(fixture.validate(&fixture.bootstrap).is_err());
    assert_eq!(fixture.current(), before);
}

#[test]
fn same_value_inode_replacement_fences_successor_preparation() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let replacement = fixture.replace_same_value();
    assert_eq!(replacement.0, *fixture.snapshot.record());
    assert_ne!(replacement.1, fixture.snapshot.file_identity());

    assert!(fixture.validate(&fixture.bootstrap).is_err());
    #[cfg(target_os = "macos")]
    assert!(fixture.install().is_err());
    assert_eq!(fixture.current(), replacement);
}

#[test]
fn mixed_successor_request_and_offer_leave_the_predecessor_exact() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let before = fixture.current();

    assert!(fixture.validate(&fixture.mixed_bootstrap()).is_err());
    assert_eq!(fixture.current(), before);
}
