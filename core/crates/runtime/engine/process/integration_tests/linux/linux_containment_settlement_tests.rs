use super::*;
use crate::process::LiveProcessIdentity;
use crate::process::install_test_match_failure;
use crate::process::inventory_test_support::TestProcess;

#[test]
fn retained_preparation_keeps_its_frozen_survivor_authority() {
    let preparation = LinuxContainmentPreparation::retained_for_test();

    assert!(matches!(
        preparation,
        LinuxContainmentPreparation::Retained(_)
    ));
}

#[test]
fn final_membership_must_equal_the_classified_survivor_generations() {
    let first = TestProcess::spawn();
    let second = TestProcess::spawn();
    let first_live = live(&first);
    let second_live = live(&second);
    let classified = HashSet::from([first_live.clone()]);

    assert_eq!(
        settlement_for_members(&classified, &classified).unwrap(),
        SettlementDecision::Retained
    );
    assert_eq!(
        settlement_for_members(
            &classified,
            &HashSet::from([first_live.clone(), second_live.clone()])
        )
        .unwrap(),
        SettlementDecision::MembershipChanged
    );
    assert_eq!(
        settlement_for_members(&HashSet::from([first_live, second_live]), &classified).unwrap(),
        SettlementDecision::MembershipChanged
    );
    assert!(first.identity.matches());
    assert!(second.identity.matches());
}

#[test]
fn unchanged_empty_membership_removes_the_containment_leaf() {
    let empty = HashSet::new();

    assert_eq!(
        settlement_for_members(&empty, &empty).unwrap(),
        SettlementDecision::Removed
    );
}

#[test]
fn dead_or_indeterminate_classified_survivor_cannot_match_a_reused_key() {
    let mut survivor = TestProcess::spawn();
    let retained = live(&survivor);
    let classified = HashSet::from([retained.clone()]);
    let same_key = HashSet::from([retained]);
    survivor.terminate();

    assert_eq!(
        settlement_for_members(&classified, &same_key).unwrap(),
        SettlementDecision::MembershipChanged
    );

    let live_survivor = TestProcess::spawn();
    let retained = live(&live_survivor);
    let classified = HashSet::from([retained.clone()]);
    let final_members = HashSet::from([retained]);
    let _failure = install_test_match_failure(SURVIVOR_IDENTITY_DETAIL);
    assert!(matches!(
        settlement_for_members(&classified, &final_members),
        Err(LinuxContainmentError::Identity(SURVIVOR_IDENTITY_DETAIL))
    ));
}

fn live(process: &TestProcess) -> LiveProcessIdentity {
    LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap()
}
