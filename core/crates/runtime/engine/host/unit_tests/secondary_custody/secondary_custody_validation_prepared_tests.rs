use super::*;
use crate::pty_custody::{CustodyDescriptorSet, CustodyDurableState};
use crate::sha256_digest::Sha256Digest;

#[path = "../../test_support/unit/secondary_custody/secondary_custody_validation_prepared.rs"]
mod support;
use support::{FOREIGN_DIGEST_BYTE, FOREIGN_READER_HANDLE, FOREIGN_WRITER_HANDLE, Fixture};

#[test]
fn prepared_guardian_accepts_current_and_proven_gone_guardians_with_exact_custody() {
    let mut fixture = Fixture::active();
    assert!(accepts(&fixture.offer, &fixture.durable));
    fixture.replace_guardian(fixture.gone);
    assert!(accepts(&fixture.offer, &fixture.durable));
}

#[test]
fn prepared_guardian_rejects_absent_predecessor_and_missing_guardian() {
    let mut fixture = Fixture::active();
    let mut pending = fixture.durable.clone();
    pending.state = CustodyDurableState::Pending {};
    assert!(!accepts(&fixture.offer, &pending));
    let mut initial = fixture.durable.clone();
    let CustodyDurableState::PreparedGuardian { predecessor, .. } = &mut initial.state else {
        unreachable!()
    };
    **predecessor = CustodyDurableState::Pending {};
    assert!(!accepts(&fixture.offer, &initial));
    fixture.candidate().guardian = None;
    assert!(!accepts(&fixture.offer, &fixture.durable));
}

#[test]
fn prepared_guardian_rejects_live_or_indeterminate_candidate_holders() {
    let mut fixture = Fixture::active();
    fixture.replace_holder(fixture.current);
    assert!(!accepts(&fixture.offer, &fixture.durable));
    fixture.replace_holder(fixture.gone);
    let _unknown = crate::process::install_test_process_identity_relation(
        fixture.gone.pid,
        fixture.gone.start_time,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert!(!accepts(&fixture.offer, &fixture.durable));
}

#[test]
fn prepared_guardian_rejects_indeterminate_guardian_without_discarding_identity() {
    let fixture = Fixture::active();
    let _unknown = crate::process::install_test_process_identity_relation(
        fixture.current.pid,
        fixture.current.start_time,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert!(!accepts(&fixture.offer, &fixture.durable));
}

#[test]
fn prepared_guardian_rejects_independent_predecessor_and_candidate_custody_changes() {
    let mut fixture = Fixture::active();
    let original = fixture.durable.clone();
    let original_offer = fixture.offer.clone();
    let foreign = CustodyDescriptorSet::macos_process_capability(
        FOREIGN_READER_HANDLE,
        FOREIGN_WRITER_HANDLE,
    );
    fixture.offer.descriptor_set = foreign;
    assert!(!accepts(&fixture.offer, &fixture.durable));
    fixture.offer = original_offer.clone();
    fixture.candidate().descriptor_set = foreign;
    assert!(!accepts(&fixture.offer, &fixture.durable));
    fixture.durable = original.clone();
    fixture.offer.recovery_identity_sha256 = Sha256Digest::repeated(FOREIGN_DIGEST_BYTE);
    assert!(!accepts(&fixture.offer, &fixture.durable));
    fixture.offer = original_offer;
    fixture.candidate().recovery_identity_sha256 = Sha256Digest::repeated(FOREIGN_DIGEST_BYTE);
    assert!(!accepts(&fixture.offer, &fixture.durable));
    assert!(accepts(&fixture.offer, &original));
}

#[test]
fn prepared_guardian_profiles_allow_only_monotonic_read_only_transition() {
    use CustodyProfile::{ActiveRunning, ActiveTerminalReadOnly, ReleasedTerminal};
    for (previous, candidate, offered, expected) in [
        (ActiveRunning, ActiveRunning, ActiveRunning, true),
        (ActiveRunning, ActiveRunning, ActiveTerminalReadOnly, true),
        (
            ActiveTerminalReadOnly,
            ActiveTerminalReadOnly,
            ActiveTerminalReadOnly,
            true,
        ),
        (ActiveTerminalReadOnly, ActiveRunning, ActiveRunning, false),
        (ActiveRunning, ActiveTerminalReadOnly, ActiveRunning, false),
    ] {
        let fixture = Fixture::new(previous, candidate, offered);
        assert_eq!(accepts(&fixture.offer, &fixture.durable), expected);
    }
    for previous in [ActiveRunning, ActiveTerminalReadOnly, ReleasedTerminal] {
        for next in [ActiveRunning, ActiveTerminalReadOnly, ReleasedTerminal] {
            let allowed = matches!(
                (previous, next),
                (ActiveRunning, ActiveRunning | ActiveTerminalReadOnly)
                    | (ActiveTerminalReadOnly, ActiveTerminalReadOnly)
            );
            assert_eq!(profile_transition(previous, next), allowed);
        }
    }
}
