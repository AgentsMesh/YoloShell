use super::*;

const IDENTITY_STEP: u64 = 1;

#[test]
fn membership_requires_captured_identity_and_current_kernel_group() {
    let current = ProcessIdentity::capture(std::process::id()).unwrap();
    let scope = PtyProcessScope::capture(current).unwrap();
    let group_id = i32::try_from(process_group(current.pid).unwrap().unwrap()).unwrap();
    assert!(scope.foreground_contains(group_id, current).unwrap());
    // The parent is live but outside the test process's owned subtree.
    let parent = ProcessIdentity::capture(unsafe { libc::getppid() } as u32).unwrap();
    assert!(!scope.foreground_contains(group_id, parent).unwrap());
    assert!(scope.foreground_contains(i32::MAX, current).is_err());
    let stale = ProcessIdentity {
        start_time: current.start_time + IDENTITY_STEP,
        ..current
    };
    assert!(!scope.foreground_contains(group_id, stale).unwrap());
}
