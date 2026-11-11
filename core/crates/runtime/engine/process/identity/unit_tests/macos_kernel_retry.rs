use super::super::{
    INACTIVE_CONFIRMATION_ATTEMPTS, MICROSECONDS_PER_SECOND, metadata_with_context,
};
use super::MAX_EXACT_METADATA_ATTEMPTS;

#[path = "../test_support/macos_kernel_retry_fixture.rs"]
mod fixture;
use fixture::{Basic, Exact, constants::*, counts, query, setup};

#[test]
fn permission_denial_retries_the_complete_record_and_preserves_every_identity_field() {
    for denied in 0..MAX_EXACT_METADATA_ATTEMPTS {
        setup(
            std::iter::repeat_n(Exact::Error(libc::EPERM), denied).chain([Exact::Active]),
            Basic::Active,
        );
        let metadata = metadata_with_context(TEST_PID, query).unwrap().unwrap();
        assert_eq!(counts(), (denied + QUERY_INCREMENT, denied));
        assert_eq!(metadata.pid, TEST_PID);
        assert_eq!(metadata.status, libc::SRUN);
        assert_eq!(metadata.parent_pid, PARENT_PID);
        assert_eq!(metadata.process_group, PROCESS_GROUP as i32);
        assert_eq!(metadata.foreground_group, FOREGROUND_GROUP as i32);
        assert_eq!(
            metadata.start_time,
            START_SECONDS * MICROSECONDS_PER_SECOND + START_MICROSECONDS
        );
        assert_eq!(metadata.unique_id, UNIQUE_ID);
        assert_eq!(metadata.parent_unique_id, PARENT_UNIQUE_ID);
        assert_eq!(metadata.pid_version, PID_VERSION as u32);
    }
}

#[test]
fn persistent_permission_denial_exhausts_the_bound_and_keeps_the_original_errno() {
    for basic in [Basic::Active, Basic::Denied] {
        setup(
            std::iter::repeat_n(Exact::Error(libc::EPERM), MAX_EXACT_METADATA_ATTEMPTS),
            basic,
        );
        let error = metadata_with_context(TEST_PID, query).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<std::io::Error>()
                .and_then(std::io::Error::raw_os_error),
            Some(libc::EPERM)
        );
        assert!(error.to_string().contains(&format!("pid={TEST_PID}")));
        assert_eq!(counts().0, MAX_EXACT_METADATA_ATTEMPTS);
        let confirmations = if matches!(basic, Basic::Denied) {
            INACTIVE_CONFIRMATION_ATTEMPTS
        } else {
            QUERY_INCREMENT
        };
        assert_eq!(counts().1, MAX_EXACT_METADATA_ATTEMPTS * confirmations);
    }
}

#[test]
fn a_denied_basic_probe_never_substitutes_for_a_successful_exact_identity_query() {
    setup([Exact::Error(libc::EPERM), Exact::Active], Basic::Denied);
    let metadata = metadata_with_context(TEST_PID, query).unwrap().unwrap();
    assert_eq!(metadata.unique_id, UNIQUE_ID);
    assert_eq!(counts(), (2, INACTIVE_CONFIRMATION_ATTEMPTS));
}

#[test]
fn retries_return_the_new_complete_generation_instead_of_merging_basic_metadata() {
    setup([Exact::Error(libc::EPERM), Exact::Changed], Basic::Active);
    let metadata = metadata_with_context(TEST_PID, query).unwrap().unwrap();
    assert_eq!(metadata.unique_id, CHANGED_UNIQUE_ID);
    assert_eq!(
        metadata.start_time,
        START_SECONDS * MICROSECONDS_PER_SECOND + CHANGED_START_MICROSECONDS
    );
    assert_eq!(metadata.pid_version, CHANGED_PID_VERSION as u32);
    assert_eq!(counts(), (2, 1));
}

#[test]
fn exec_transition_uses_new_pid_version_without_replacing_the_process_identity() {
    setup([Exact::Error(libc::EPERM), Exact::Executed], Basic::Active);
    let metadata = metadata_with_context(TEST_PID, query).unwrap().unwrap();
    assert_eq!(metadata.pid, TEST_PID);
    assert_eq!(metadata.unique_id, UNIQUE_ID);
    assert_eq!(metadata.parent_unique_id, PARENT_UNIQUE_ID);
    assert_eq!(
        metadata.start_time,
        START_SECONDS * MICROSECONDS_PER_SECOND + START_MICROSECONDS
    );
    assert_eq!(metadata.pid_version, CHANGED_PID_VERSION as u32);
    assert_eq!(counts(), (2, 1));
}

#[test]
fn malformed_success_or_nonpermission_error_stops_retries_without_inventing_absence() {
    for outcome in [
        Exact::WrongPid,
        Exact::MissingUnique,
        Exact::Short,
        Exact::NoErrno,
        Exact::Error(libc::EIO),
        Exact::Error(libc::EINVAL),
        Exact::Error(libc::ENOENT),
    ] {
        setup(
            [Exact::Error(libc::EPERM), outcome, Exact::Active],
            Basic::Active,
        );
        assert!(metadata_with_context(TEST_PID, query).is_err());
        assert_eq!(counts().0, 2);
    }
}

#[test]
fn only_explicit_absence_or_complete_zombie_metadata_can_end_a_retry_as_gone() {
    for outcome in [Exact::Zombie, Exact::Error(libc::ESRCH)] {
        setup([Exact::Error(libc::EPERM), outcome], Basic::Active);
        assert!(metadata_with_context(TEST_PID, query).unwrap().is_none());
        assert_eq!(counts().0, 2);
    }
    setup([Exact::Error(libc::EPERM), Exact::Active], Basic::Gone);
    assert!(metadata_with_context(TEST_PID, query).unwrap().is_none());
    assert_eq!(counts(), (1, 1));
}

#[test]
fn invalid_pid_is_rejected_before_any_kernel_query() {
    for pid in [u32::MIN, u32::MAX] {
        setup([Exact::Active], Basic::Active);
        assert!(metadata_with_context(pid, query).is_err());
        assert_eq!(counts(), (0, 0));
    }
}
