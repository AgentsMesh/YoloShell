use super::super::LiveProcessIdentity;
use super::*;
use crate::process::ProcessIdentity;
use std::process::{Child, ExitStatus};

const OUT_OF_RANGE_PID: u32 = u32::MAX;
const NEXT_GENERATION_OFFSET: u64 = 1;
const SLEEP_DURATION: &str = "30";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";

struct ChildGuard {
    child: Option<Child>,
    identity: LiveProcessIdentity,
}

impl ChildGuard {
    fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            std::process::Command::new(SLEEP_EXECUTABLE).arg(SLEEP_DURATION),
        );
        let durable = ProcessIdentity::capture(child.id()).unwrap();
        let identity = LiveProcessIdentity::capture(durable).unwrap().unwrap();
        Self {
            child: Some(child),
            identity,
        }
    }

    fn reap(&mut self) -> ExitStatus {
        self.child.take().unwrap().wait().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = self.identity.signal(libc::SIGKILL);
            let _ = child.wait();
        }
    }
}

#[test]
fn kernel_pidfd_delivers_to_the_captured_identity() {
    let mut child = ChildGuard::spawn();

    assert!(child.identity.signal(libc::SIGTERM).unwrap());
    assert!(!child.reap().success());
}

#[test]
fn retained_authority_rejects_a_forged_durable_projection() {
    let mut child = ChildGuard::spawn();
    let generation = child
        .identity
        .lineage_id()
        .wrapping_add(NEXT_GENERATION_OFFSET);
    let forged = child.identity.with_generation(generation);

    assert!(!forged.signal(libc::SIGKILL).unwrap());
    assert!(child.identity.matches());
    assert!(child.identity.signal(libc::SIGKILL).unwrap());
    child.reap();
}

#[test]
fn out_of_range_pid_is_rejected_before_the_syscall() {
    let error = KernelPidfdOperations.open(OUT_OF_RANGE_PID).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn only_absence_errors_are_classified_as_process_exit() {
    let exited = io::Error::from_raw_os_error(libc::ESRCH);
    let denied = io::Error::from_raw_os_error(libc::EPERM);
    let procfs_missing: anyhow::Error = io::Error::from_raw_os_error(libc::ENOENT).into();
    let procfs_exited: anyhow::Error = io::Error::from_raw_os_error(libc::ESRCH).into();
    let procfs_denied: anyhow::Error = io::Error::from_raw_os_error(libc::EACCES).into();

    assert!(is_process_gone(&exited));
    assert!(!is_process_gone(&denied));
    assert!(error_is_process_gone(&procfs_missing));
    assert!(error_is_process_gone(&procfs_exited));
    assert!(!error_is_process_gone(&procfs_denied));
}
