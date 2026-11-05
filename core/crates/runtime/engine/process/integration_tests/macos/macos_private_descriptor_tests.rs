use super::*;
use std::os::fd::AsRawFd;
use std::process::Command;

const TEST_CONTEXT: &str = "create test private kqueue";
const SHELL: &str = "/bin/sh";
const SHELL_COMMAND: &str = "-c";
const DESCRIPTOR_ENV: &str = "YOLO_TEST_PRIVATE_DESCRIPTOR";
const ABSENCE_SCRIPT: &str = "test ! -e /dev/fd/$YOLO_TEST_PRIVATE_DESCRIPTOR";

#[test]
fn private_kqueue_is_absent_from_a_real_exec_child_descriptor_census() {
    let queue = private_kqueue(TEST_CONTEXT).unwrap();
    let flags = unsafe { libc::fcntl(queue.as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());

    let status = Command::new(SHELL)
        .args([SHELL_COMMAND, ABSENCE_SCRIPT])
        .env(DESCRIPTOR_ENV, queue.as_raw_fd().to_string())
        .status()
        .unwrap();

    assert!(status.success());
}

#[test]
fn guarded_private_kqueue_is_cloexec_before_spawn_boundary_release() {
    let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let _reject_reentry = crate::process::reject_spawn_gate_entry_for_test();
    let queue = private_kqueue_in_spawn_boundary(TEST_CONTEXT, &spawn_guard.boundary()).unwrap();
    let flags = unsafe { libc::fcntl(queue.as_raw_fd(), libc::F_GETFD) };

    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}
