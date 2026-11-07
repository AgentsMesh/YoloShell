use super::test_support::*;
use super::*;
use std::collections::BTreeMap;

#[test]
fn invalid_initial_pid_start_time_or_pinned_path_is_rejected() {
    let invalid = [
        ProcessIdentity {
            pid: TEST_PID + u32::try_from(IDENTITY_CHANGE).unwrap(),
            ..identity()
        },
        ProcessIdentity {
            start_time: u64::default(),
            ..identity()
        },
        ProcessIdentity {
            cwd: CwdIdentity {
                path: TEST_RELATIVE_CWD.into(),
                ..identity().cwd
            },
            ..identity()
        },
        ProcessIdentity {
            executable_file: ExecutableIdentity {
                path: TEST_RELATIVE_CWD.into(),
                ..identity().executable_file
            },
            ..identity()
        },
        ProcessIdentity {
            executable_file: ExecutableIdentity {
                file_type: libc::S_IFDIR as u32,
                ..identity().executable_file
            },
            ..identity()
        },
    ];
    for before in invalid {
        let identities = [Some(before.clone()), Some(before)];
        assert_rejected(identities, row());
    }
    assert!(
        observe_process(
            &api([None], EnvironmentOutcome::Unavailable, Vec::new()),
            row(),
            &[],
        )
        .is_none()
    );
    let mut zero_pid = identity();
    zero_pid.pid = u32::default();
    let mut zero_pid_row = row();
    zero_pid_row.pid = u32::default();
    assert_rejected([Some(zero_pid.clone()), Some(zero_pid)], zero_pid_row);
}

#[test]
fn pid_start_executable_and_cwd_races_fail_closed() {
    let changed = [
        ProcessIdentity {
            pid: TEST_PID + u32::try_from(IDENTITY_CHANGE).unwrap(),
            ..identity()
        },
        ProcessIdentity {
            start_time: TEST_START_TIME + IDENTITY_CHANGE,
            ..identity()
        },
        ProcessIdentity {
            process_inode: TEST_PROCESS_INODE + IDENTITY_CHANGE,
            ..identity()
        },
        ProcessIdentity {
            process_device: TEST_PROCESS_DEVICE + IDENTITY_CHANGE,
            ..identity()
        },
        ProcessIdentity {
            executable_file: ExecutableIdentity {
                inode: TEST_PROCESS_INODE + IDENTITY_CHANGE,
                ..identity().executable_file
            },
            ..identity()
        },
        ProcessIdentity {
            executable_file: ExecutableIdentity {
                path: TEST_OTHER_CWD.into(),
                ..identity().executable_file
            },
            ..identity()
        },
        ProcessIdentity {
            cwd: CwdIdentity {
                path: TEST_OTHER_CWD.into(),
                ..identity().cwd
            },
            ..identity()
        },
        ProcessIdentity {
            cwd: CwdIdentity {
                inode: TEST_INODE + IDENTITY_CHANGE,
                ..identity().cwd
            },
            ..identity()
        },
        ProcessIdentity {
            cwd: CwdIdentity {
                device: TEST_DEVICE + IDENTITY_CHANGE,
                ..identity().cwd
            },
            ..identity()
        },
    ];
    for after in changed {
        assert_rejected([Some(identity()), Some(after)], row());
    }
    assert_rejected([Some(identity()), None], row());
}

fn assert_rejected(
    identities: impl IntoIterator<Item = Option<ProcessIdentity>>,
    process: ProcessRow,
) {
    assert!(
        observe_process(
            &api(
                identities,
                EnvironmentOutcome::Selected(BTreeMap::new()),
                Vec::new(),
            ),
            process,
            &[],
        )
        .is_none()
    );
}
