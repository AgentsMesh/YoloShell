use super::*;
use crate::fixture_constants as fixture;
use std::process::Command;
const WAIT_FOR_EXIT: libc::c_int = 0;

#[test]
fn externally_reaped_child_is_not_cleanup_success() {
    let child = Command::new(fixture::TEST_EXIT_EXECUTABLE).spawn().unwrap();
    let process_id = child.id();
    let mut status = libc::c_int::default();
    let waited = unsafe { libc::waitpid(process_id as libc::pid_t, &mut status, WAIT_FOR_EXIT) };
    assert_eq!(waited, process_id as libc::pid_t);

    let error = PendingBroker::detached_for_test(child)
        .compensate()
        .unwrap_err();

    let failure = error.downcast_ref::<ChildSettlementFailure>().unwrap();
    assert!(failure.wait_authority_lost());
    assert!(
        error
            .to_string()
            .contains(super::failure::WAIT_AUTHORITY_LOST_CONTEXT)
    );
}
