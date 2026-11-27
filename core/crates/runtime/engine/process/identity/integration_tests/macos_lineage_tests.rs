use super::*;
use std::collections::HashSet;

const SYSTEM_INIT_PID: u32 = 1;

#[test]
fn privileged_system_init_has_a_terminal_lineage() {
    let init = capture(SYSTEM_INIT_PID).unwrap().unwrap();
    assert_eq!(init.pid, SYSTEM_INIT_PID);
    assert_eq!(init.parent_pid, 0);
    assert_ne!(init.unique_id, 0);
}

#[test]
fn invalid_pids_fail_and_absent_pids_remain_absent() {
    assert!(capture(0).is_err());
    assert!(capture(u32::MAX).is_err());
    assert!(capture(libc::c_int::MAX as u32).unwrap().is_none());
}

#[test]
fn current_ancestry_can_be_captured_through_privileged_login_parents() {
    let mut current = capture(std::process::id()).unwrap().unwrap();
    let mut seen = HashSet::new();
    loop {
        assert!(seen.insert(current.unique_id));
        if current.parent_pid == SYSTEM_INIT_PID || current.parent_pid == 0 {
            break;
        }
        let parent = capture(current.parent_pid).unwrap().unwrap();
        assert_eq!(parent.unique_id, current.parent_unique_id);
        current = parent;
    }
}
