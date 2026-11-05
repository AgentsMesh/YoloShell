use super::*;
use crate::parent_test_support::*;

#[test]
fn known_callback_shell_paths_are_skipped_on_both_platforms() {
    for shell in NATIVE_CALLBACK_SHELLS {
        let processes = lineage(shell);
        let result = candidate_from(HELPER_PID, |pid| {
            processes
                .get(&pid)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!(MISSING_PARENT))
        })
        .unwrap();
        assert_eq!(
            result,
            ClaudeProcessIdentity {
                pid: AGENT_PID,
                start_time: START_TIME
            }
        );
    }
}

#[test]
fn direct_agent_parent_needs_no_shell_or_launch_registration() {
    let mut processes = lineage(NATIVE_CALLBACK_SHELLS[0]);
    processes.get_mut(&HELPER_PID).unwrap().parent_pid = AGENT_PID;
    let result = candidate_from(HELPER_PID, |pid| {
        processes
            .get(&pid)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(MISSING_PARENT))
    })
    .unwrap();
    assert_eq!(result.pid, AGENT_PID);
}

#[test]
fn reused_child_identity_during_ancestry_capture_is_rejected() {
    let processes = lineage(NATIVE_CALLBACK_SHELLS[0]);
    let mut observed_helper = false;
    assert!(
        candidate_from(HELPER_PID, |pid| {
            let mut process = processes.get(&pid).cloned().unwrap();
            if pid == HELPER_PID {
                if observed_helper {
                    process.start_time = OTHER_START;
                }
                observed_helper = true;
            }
            Ok(process)
        })
        .is_err()
    );
}

#[test]
fn cyclic_missing_and_invalid_parent_observations_are_rejected() {
    for (parent_pid, parent_start) in [
        (HELPER_PID, START_TIME),
        (SHELL_PID, u64::default()),
        (u32::default(), START_TIME),
    ] {
        let mut processes = lineage(NATIVE_CALLBACK_SHELLS[0]);
        let shell = processes.get_mut(&SHELL_PID).unwrap();
        shell.parent_pid = parent_pid;
        shell.start_time = parent_start;
        assert!(
            candidate_from(HELPER_PID, |pid| processes
                .get(&pid)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!(MISSING_PARENT)))
            .is_err()
        );
    }
    assert!(candidate_from(HELPER_PID, |_| anyhow::bail!(MISSING_PARENT)).is_err());
}

#[test]
fn ancestry_walk_has_a_fixed_upper_bound() {
    assert!(
        candidate_from(HELPER_PID, |pid| Ok(process(
            pid,
            pid + 1,
            NATIVE_CALLBACK_SHELLS[0]
        )))
        .is_err()
    );
}

#[test]
fn a_non_shell_is_returned_for_worker_authentication_not_skipped() {
    let processes = lineage(HELPER);
    let result = candidate_from(HELPER_PID, |pid| {
        processes
            .get(&pid)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(MISSING_PARENT))
    })
    .unwrap();
    assert_eq!(result.pid, SHELL_PID);
}
