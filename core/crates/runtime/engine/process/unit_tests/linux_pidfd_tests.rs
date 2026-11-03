use super::*;
const TEST_SIGNAL: i32 = libc::SIGTERM;
include!("../test_support/linux/linux_pidfd_operations.rs");

#[test]
fn capture_opens_once_and_later_signal_uses_the_retained_descriptor() {
    let operations = FakeOperations::successful();
    let descriptor = capture_with(&operations, identity()).unwrap().unwrap();

    assert!(signal_with(&operations, &descriptor, TEST_SIGNAL).unwrap());
    assert_eq!(
        operations.calls(),
        vec![
            Call::Open(EXPECTED_PID),
            Call::StartTime(EXPECTED_PID),
            Call::Exited,
            Call::Send(TEST_SIGNAL),
        ]
    );
}

#[test]
fn changed_or_exited_candidate_is_not_retained() {
    let scenarios = [
        FakeOperations {
            start_time: StartTimeOutcome::Value(Some(REPLACEMENT_START_TIME)),
            ..FakeOperations::successful()
        },
        FakeOperations {
            exited: Ok(true),
            ..FakeOperations::successful()
        },
    ];
    for operations in scenarios {
        assert!(capture_with(&operations, identity()).unwrap().is_none());
    }
}

#[test]
fn candidate_disappearance_is_local_but_capture_failures_are_errors() {
    let disappeared = [
        FakeOperations {
            open: IoOutcome::OsError(libc::ESRCH),
            ..FakeOperations::successful()
        },
        FakeOperations {
            start_time: StartTimeOutcome::Value(None),
            ..FakeOperations::successful()
        },
    ];
    for operations in disappeared {
        assert!(capture_with(&operations, identity()).unwrap().is_none());
    }

    let failures = [
        (
            FakeOperations {
                open: IoOutcome::OsError(libc::ENOSYS),
                ..FakeOperations::successful()
            },
            OPEN_CONTEXT,
        ),
        (
            FakeOperations {
                start_time: StartTimeOutcome::Failure,
                ..FakeOperations::successful()
            },
            START_TIME_CONTEXT,
        ),
        (
            FakeOperations {
                exited: Err(io::Error::from_raw_os_error(libc::EBADF)),
                ..FakeOperations::successful()
            },
            POLL_CONTEXT,
        ),
    ];
    for (operations, context) in failures {
        let error = capture_with(&operations, identity()).unwrap_err();
        assert!(format!("{error:#}").contains(context));
    }
}

#[test]
fn retained_signal_disappearance_is_noop_and_other_failures_fail_closed() {
    let disappeared = FakeOperations {
        send: IoOutcome::OsError(libc::ESRCH),
        ..FakeOperations::successful()
    };
    assert!(!signal_with(&disappeared, &TEST_DESCRIPTOR, TEST_SIGNAL).unwrap());

    let denied = FakeOperations {
        send: IoOutcome::OsError(libc::EPERM),
        ..FakeOperations::successful()
    };
    let error = signal_with(&denied, &TEST_DESCRIPTOR, TEST_SIGNAL).unwrap_err();
    assert!(format!("{error:#}").contains(SIGNAL_CONTEXT));
}
