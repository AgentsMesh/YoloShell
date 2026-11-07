use super::*;
#[cfg(target_os = "macos")]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_recovery_admission_test_support.rs"]
mod admission_support;
#[cfg(target_os = "macos")]
use admission_support::{FenceState, RecoveryAdmissionFixture};

const PID_IDENTITY_DRIFT_MASK: u32 = 1;
const START_TIME_IDENTITY_DRIFT_MASK: u64 = 1;
const BOOT_IDENTITY_DRIFT_SUFFIX: &str = "-drifted";

#[derive(Clone, Copy)]
enum RetirementIdentityDrift {
    WorkerPid,
    WorkerStartTime,
    ChildPid,
    ChildStartTime,
    BootId,
}

impl RetirementIdentityDrift {
    fn apply(self, runtime: &mut CustodyRuntimeIdentity) {
        match self {
            Self::WorkerPid => runtime.worker_pid ^= PID_IDENTITY_DRIFT_MASK,
            Self::WorkerStartTime => {
                runtime.worker_start_time ^= START_TIME_IDENTITY_DRIFT_MASK;
            }
            Self::ChildPid => runtime.child_pid ^= PID_IDENTITY_DRIFT_MASK,
            Self::ChildStartTime => {
                runtime.child_start_time ^= START_TIME_IDENTITY_DRIFT_MASK;
            }
            Self::BootId => runtime.boot_id.push_str(BOOT_IDENTITY_DRIFT_SUFFIX),
        }
    }
}

#[test]
#[cfg(target_os = "macos")]
fn live_guardian_fences_remain_awaiting_settlement() {
    for state in [FenceState::PreparedGuardian, FenceState::ArmedSuccessor] {
        let fixture =
            RecoveryAdmissionFixture::new(state, crate::process::ProcessIdentityRelation::Matching);

        assert_eq!(
            fixture.admission(),
            SecondaryCustodyRecoveryAdmission::AwaitingSettlement
        );
    }
}

#[test]
#[cfg(target_os = "macos")]
fn indeterminate_prepared_guardian_remains_unavailable() {
    let fixture = RecoveryAdmissionFixture::new(
        FenceState::PreparedGuardian,
        crate::process::ProcessIdentityRelation::Unknown,
    );

    assert_eq!(
        fixture.admission(),
        SecondaryCustodyRecoveryAdmission::Unavailable
    );
}

#[tokio::test]
async fn retained_retirement_only_waits_while_acknowledgement_can_settle() {
    for (state, expected) in [
        (
            TestRetiredCustodyState::ReleaseAckPending,
            SecondaryCustodyRecoveryAdmission::AwaitingSettlement,
        ),
        (
            TestRetiredCustodyState::ReleaseUncertain,
            SecondaryCustodyRecoveryAdmission::Unavailable,
        ),
    ] {
        let directory = tempdir().unwrap();
        let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
        let record = runtime(&host, directory.path().join(TEST_SOCKET_NAME));
        host.secondary_custody
            .install_test_retired_lease(&host, &record, state);

        assert_eq!(
            host.secondary_custody
                .recovery_admission(&host, &CustodyRuntimeIdentity::from_record(&record)),
            expected
        );
    }
}

#[tokio::test]
async fn release_ack_pending_requires_the_entire_runtime_identity() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let record = runtime(&host, directory.path().join(TEST_SOCKET_NAME));
    let exact = CustodyRuntimeIdentity::from_record(&record);
    let exact_key = SecondaryCustodyKey::from_runtime(&exact);
    host.secondary_custody.install_test_retired_lease(
        &host,
        &record,
        TestRetiredCustodyState::ReleaseAckPending,
    );

    assert_eq!(
        host.secondary_custody.recovery_admission(&host, &exact),
        SecondaryCustodyRecoveryAdmission::AwaitingSettlement
    );
    for drift in [
        RetirementIdentityDrift::WorkerPid,
        RetirementIdentityDrift::WorkerStartTime,
        RetirementIdentityDrift::ChildPid,
        RetirementIdentityDrift::ChildStartTime,
        RetirementIdentityDrift::BootId,
    ] {
        let mut observed = exact.clone();
        drift.apply(&mut observed);

        assert_eq!(SecondaryCustodyKey::from_runtime(&observed), exact_key);
        assert_ne!(observed, exact);
        assert_eq!(
            host.secondary_custody.recovery_admission(&host, &observed),
            SecondaryCustodyRecoveryAdmission::Unavailable
        );
    }
}
