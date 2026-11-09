use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

const ATTACHMENT_AUTHORITY_SOCKET_NAME: &str = "released-attachment-authority.sock";
const ALTERED_CHILD_START_TIME_INCREMENT: u64 = 1;
const SETTLED_CHILD_PID: u32 = u32::MAX;
const SETTLED_CHILD_START_TIME: u64 = 1;
const RETIRED_LOCATION: &str = "location=Retired";
const ACTIVE_LOCATION: &str = "location=Active";
const ABSENT_LOCATION: &str = "location=Absent";
const MATCHING_DIAGNOSTIC: &str = "matches_runtime=true";
const MISMATCHED_DIAGNOSTIC: &str = "matches_runtime=false";

#[tokio::test]
async fn an_active_released_state_is_diagnostic_not_retirement_authority() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let record = runtime(
        &host,
        directory.path().join(ATTACHMENT_AUTHORITY_SOCKET_NAME),
    );
    let runtime = CustodyRuntimeIdentity::from_record(&record);
    let absent = host.secondary_custody.diagnostic_state(&runtime);
    assert!(absent.contains(ABSENT_LOCATION), "{absent}");
    let lease = host
        .secondary_custody
        .install_test_active_lease(&host, &record);
    lease.set_state(SecondaryCustodyState::Released);

    let diagnostic = host.secondary_custody.diagnostic_state(&runtime);

    assert!(diagnostic.contains(ACTIVE_LOCATION), "{diagnostic}");
    assert!(diagnostic.contains(MATCHING_DIAGNOSTIC), "{diagnostic}");
    assert!(!diagnostic.contains(&record.auth_token));
    assert!(
        !host
            .secondary_custody
            .released_for_read_only_attachment(&runtime)
    );
    assert!(
        !host
            .secondary_custody
            .released_for_terminal_worker_shutdown(&runtime)
    );
}

#[tokio::test]
async fn read_only_and_shutdown_authority_require_an_exact_released_runtime() {
    for (state, expected) in [
        (TestRetiredCustodyState::ReleaseAckPending, false),
        (TestRetiredCustodyState::Released, true),
        (TestRetiredCustodyState::ReleaseUncertain, false),
        (TestRetiredCustodyState::Superseded, false),
    ] {
        let directory = tempdir().unwrap();
        let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
        let mut record = runtime(
            &host,
            directory.path().join(ATTACHMENT_AUTHORITY_SOCKET_NAME),
        );
        if matches!(state, TestRetiredCustodyState::Released) {
            record.child_pid = SETTLED_CHILD_PID;
            record.child_start_time = SETTLED_CHILD_START_TIME;
        }
        let runtime = CustodyRuntimeIdentity::from_record(&record);
        let _lease = host
            .secondary_custody
            .install_test_retired_lease(&host, &record, state);
        let diagnostic = host.secondary_custody.diagnostic_state(&runtime);
        assert!(diagnostic.contains(RETIRED_LOCATION), "{diagnostic}");
        assert!(diagnostic.contains(MATCHING_DIAGNOSTIC), "{diagnostic}");
        assert!(!diagnostic.contains(&record.auth_token));

        assert_eq!(
            host.secondary_custody
                .released_for_read_only_attachment(&runtime),
            expected
        );
        assert_eq!(
            host.secondary_custody
                .released_for_terminal_worker_shutdown(&runtime),
            expected
        );

        let mut altered = runtime.clone();
        altered.child_start_time = altered
            .child_start_time
            .saturating_add(ALTERED_CHILD_START_TIME_INCREMENT);
        let diagnostic = host.secondary_custody.diagnostic_state(&altered);
        assert!(diagnostic.contains(MISMATCHED_DIAGNOSTIC), "{diagnostic}");
        assert!(
            !host
                .secondary_custody
                .released_for_read_only_attachment(&altered)
        );
        assert!(
            !host
                .secondary_custody
                .released_for_terminal_worker_shutdown(&altered)
        );
    }
}
