#![cfg(test)]

use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use crate::process::ProcessIdentity;
use std::ffi::OsStr;
use std::process::Command;

const HELPER_TEST: &str =
    "custody_guardian::settlement_failure::containment_tests::stopped_child_helper";
const HELPER_ROLE_ENV: &str = "YOLO_GUARDIAN_STOPPED_CHILD_ROLE";
const HELPER_ROLE: &str = "stopped-child";

pub(super) fn command() -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(HELPER_ROLE_ENV, HELPER_ROLE);
    command
}

pub(super) fn run() {
    if std::env::var_os(HELPER_ROLE_ENV).as_deref() != Some(OsStr::new(HELPER_ROLE)) {
        return;
    }
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
    let current = ProcessIdentity::capture(std::process::id()).unwrap();
    assert!(current.signal(libc::SIGSTOP).unwrap());
    loop {
        std::thread::park();
    }
}
