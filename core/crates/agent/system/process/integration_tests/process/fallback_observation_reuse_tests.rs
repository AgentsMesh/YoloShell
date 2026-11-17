use super::test_support::*;
use super::{ProcessIdentity, observe_process};
use std::collections::BTreeMap;

#[test]
fn reuse_after_inventory_is_rejected_even_when_the_new_identity_stays_stable() {
    let replacements = [
        ProcessIdentity {
            start_time: TEST_START_TIME + IDENTITY_CHANGE,
            ..identity()
        },
        ProcessIdentity {
            parent_pid: TEST_PID,
            ..identity()
        },
        ProcessIdentity {
            process_group: TEST_PROCESS_GROUP + i32::try_from(IDENTITY_CHANGE).unwrap(),
            ..identity()
        },
        ProcessIdentity {
            foreground_group: TEST_FOREGROUND_GROUP + i32::try_from(IDENTITY_CHANGE).unwrap(),
            ..identity()
        },
        ProcessIdentity {
            executable: crate::agent::claude::constants::EXECUTABLE.to_owned(),
            ..identity()
        },
    ];
    for replacement in replacements {
        let stable_replacement = [Some(replacement.clone()), Some(replacement)];
        assert!(
            observe_process(
                &api(
                    stable_replacement,
                    EnvironmentOutcome::Selected(BTreeMap::new()),
                    Vec::new(),
                ),
                row(),
                &[],
            )
            .is_none()
        );
    }
}
