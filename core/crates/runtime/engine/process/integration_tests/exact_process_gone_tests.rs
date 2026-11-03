use super::*;

const VALID_PID: u32 = 42_101;
const SUBSTITUTE_PID: u32 = 42_102;
const VALID_START_TIME: u64 = 52_101;
const SUBSTITUTE_START_TIME: u64 = 52_102;
const FIRST_OUT_OF_RANGE_PID: u32 = i32::MAX as u32 + 1;
const KNOWN_BOOT_ID: &str = "test-exact-process-gone-boot";
const UNKNOWN_BOOT_ID: &str = "unknown:test-exact-process-gone-boot";
const GONE_PROOF_EXPECTED: &str = "exact Gone observation did not produce a proof";
const TEST_GONE_PROOF_EXPECTED: &str = "controlled Gone observation did not produce a proof";
const INVALID_BOOT_OBSERVATION_PANIC: &str =
    "process observation ran without stable boot authority";

#[test]
fn exact_gone_observation_binds_the_full_identity_and_boot() {
    let identity = valid_identity();
    let boot = known_boot();
    let proof =
        ExactProcessGone::capture_with(identity, boot, BootIdentityRelation::Same, |observed| {
            assert_eq!(observed, identity);
            ProcessIdentityObservation::Gone
        })
        .expect(GONE_PROOF_EXPECTED);

    assert_eq!(proof.identity(), identity);
    assert_eq!(proof.boot_identity(), boot);
    assert_ne!(proof.identity(), pid_substitution());
    assert_ne!(proof.identity(), start_time_substitution());
}

#[test]
fn matching_and_unknown_observations_fail_closed() {
    for observation in [
        ProcessIdentityObservation::Matching,
        ProcessIdentityObservation::Unknown,
    ] {
        assert!(
            ExactProcessGone::capture_with(
                valid_identity(),
                known_boot(),
                BootIdentityRelation::Same,
                |_| observation,
            )
            .is_none()
        );
    }
}

#[test]
fn controlled_test_observation_retains_production_validation() {
    let proof =
        ExactProcessGone::capture_for_test(valid_identity(), ProcessIdentityObservation::Gone)
            .expect(TEST_GONE_PROOF_EXPECTED);
    assert_eq!(proof.identity(), valid_identity());
    assert!(proof.boot_identity().known());
    assert_eq!(
        proof.boot_identity().relation_to_current(),
        BootIdentityRelation::Same
    );

    for observation in [
        ProcessIdentityObservation::Matching,
        ProcessIdentityObservation::Unknown,
    ] {
        assert!(ExactProcessGone::capture_for_test(valid_identity(), observation).is_none());
    }
}

#[test]
fn unknown_or_noncurrent_boot_fails_before_process_observation() {
    assert_invalid_boot(
        BootIdentity::from_value(UNKNOWN_BOOT_ID),
        BootIdentityRelation::Same,
    );
    for relation in [
        BootIdentityRelation::Different,
        BootIdentityRelation::Unknown,
    ] {
        assert_invalid_boot(known_boot(), relation);
    }
}

#[test]
fn invalid_process_identity_cannot_receive_test_or_internal_proof() {
    for identity in [
        ProcessIdentity {
            pid: u32::default(),
            start_time: VALID_START_TIME,
        },
        ProcessIdentity {
            pid: FIRST_OUT_OF_RANGE_PID,
            start_time: VALID_START_TIME,
        },
        ProcessIdentity {
            pid: VALID_PID,
            start_time: u64::default(),
        },
    ] {
        assert!(
            ExactProcessGone::capture_with(
                identity,
                known_boot(),
                BootIdentityRelation::Same,
                |_| ProcessIdentityObservation::Gone,
            )
            .is_none()
        );
        assert!(
            ExactProcessGone::capture_for_test(identity, ProcessIdentityObservation::Gone)
                .is_none()
        );
    }
}

#[test]
fn production_observer_rejects_the_current_matching_process() {
    let current = ProcessIdentity::capture(std::process::id()).unwrap();
    assert!(ExactProcessGone::capture(current).is_none());
}

fn assert_invalid_boot(boot: BootIdentity, relation: BootIdentityRelation) {
    let proof = ExactProcessGone::capture_with(valid_identity(), boot, relation, |_| {
        panic!("{INVALID_BOOT_OBSERVATION_PANIC}")
    });
    assert!(proof.is_none());
}

fn valid_identity() -> ProcessIdentity {
    ProcessIdentity {
        pid: VALID_PID,
        start_time: VALID_START_TIME,
    }
}

fn pid_substitution() -> ProcessIdentity {
    ProcessIdentity {
        pid: SUBSTITUTE_PID,
        ..valid_identity()
    }
}

fn start_time_substitution() -> ProcessIdentity {
    ProcessIdentity {
        start_time: SUBSTITUTE_START_TIME,
        ..valid_identity()
    }
}

fn known_boot() -> BootIdentity {
    BootIdentity::from_value(KNOWN_BOOT_ID)
}
