use super::*;
use crate::process::inventory_test_support::TestProcess;

const GENERATION_STEP: u64 = 1;
const OBSERVED_PID: u32 = 101;
const OBSERVED_GENERATION: u64 = 200;

#[test]
fn external_exit_between_member_snapshot_and_pidfd_capture_requires_retry() {
    let mut child = TestProcess::spawn();
    let identity = child.identity;
    let observed = capture_member(identity.pid, process_start_time, |expected| {
        assert_eq!(expected, identity);
        child.terminate();
        LiveProcessIdentity::capture(expected)
    })
    .unwrap();
    assert!(observed.is_none());
    assert!(
        capture_members(&BTreeSet::from([identity.pid]))
            .unwrap()
            .is_none()
    );
    assert!(
        capture_members(&BTreeSet::new())
            .unwrap()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn generation_replacement_and_unavailable_identity_are_not_treated_as_exit() {
    let mut observations = [OBSERVED_GENERATION, OBSERVED_GENERATION + GENERATION_STEP].into_iter();
    let replaced = capture_member(
        OBSERVED_PID,
        |_| Ok(observations.next().unwrap()),
        |_| Ok(None),
    );
    assert!(matches!(
        replaced,
        Err(LinuxContainmentError::Identity(MEMBER_REPLACED_DETAIL))
    ));
    let denied = capture_member(
        OBSERVED_PID,
        |_| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into()),
        |_| panic!("unobserved member must not receive pidfd authority"),
    );
    assert!(matches!(
        denied,
        Err(LinuxContainmentError::Identity(MEMBER_IDENTITY_DETAIL))
    ));
}

#[test]
fn sustained_membership_instability_keeps_one_bounded_deadline() {
    let now = Instant::now();
    let mut started = None;
    assert!(pending_membership(&mut started, now).unwrap().is_none());
    assert_eq!(started, Some(now));
    assert!(
        pending_membership(&mut started, now + FREEZE_TIMEOUT / 2)
            .unwrap()
            .is_none()
    );
    assert_eq!(started, Some(now));
    assert!(matches!(
        pending_membership(&mut started, now + FREEZE_TIMEOUT),
        Err(LinuxContainmentError::Unavailable(
            MEMBERSHIP_TIMEOUT_DETAIL
        ))
    ));
    assert_eq!(started, Some(now));
}
