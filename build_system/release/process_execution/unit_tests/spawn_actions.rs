use super::*;
use crate::unit_support::*;
use std::os::fd::AsRawFd;

#[test]
fn native_spawn_actions_are_validated_without_starting_a_process() {
    let file = tempfile::tempfile().unwrap();
    let mut actions = FileActions::new().unwrap();
    actions.open_null_stdin().unwrap();
    actions.fchdir(file.as_raw_fd()).unwrap();
    actions.dup(file.as_raw_fd(), libc::STDOUT_FILENO).unwrap();
    assert!(
        actions
            .dup(INVALID_DESCRIPTOR, libc::STDOUT_FILENO)
            .is_err()
    );
    assert!(actions.dup(file.as_raw_fd(), INVALID_DESCRIPTOR).is_err());
    assert!(actions.fchdir(INVALID_DESCRIPTOR).is_err());
    SpawnAttributes::new().unwrap().configure().unwrap();
}

#[test]
fn spawn_status_checks_keep_context_and_os_error() {
    check(EMPTY_WAIT_STATUS, DESCRIPTION).unwrap();
    let error = check(libc::EINVAL, DESCRIPTION).unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::EINVAL)
    );
    assert!(error.to_string().contains(DESCRIPTION));
}
