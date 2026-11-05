use super::constants::PROCESS_INVENTORY_CAPTURE_ATTEMPTS;
use super::inventory_snapshot::install_test_inventory_results;
use super::resilience_adapter::{send_signal, terminate_process_tree};
use super::{DetachedStandardError, is_process_alive, spawn_detached};
use std::io;
use std::path::Path;
use std::process::Child;

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_SECONDS: &str = "30";
const MISSING_PROCESS_ID: u32 = libc::pid_t::MAX as u32;
const INVENTORY_FAILURE_DETAIL: &str = "injected resilience inventory failure";

#[test]
fn resilience_signaling_treats_a_missing_process_as_settled() {
    send_signal(MISSING_PROCESS_ID, libc::SIGCONT).unwrap();
    terminate_process_tree(MISSING_PROCESS_ID, false).unwrap();
    terminate_process_tree(MISSING_PROCESS_ID, true).unwrap();
}

#[test]
fn resilience_signaling_observes_a_live_process_identity() {
    let mut child = ChildGuard::spawn();
    let pid = child.id();

    send_signal(pid, libc::SIGCONT).unwrap();

    assert!(is_process_alive(pid));
    terminate_process_tree(pid, false).unwrap();
    assert!(!child.wait().success());
}

#[test]
fn resilience_force_termination_kills_the_captured_tree() {
    let mut child = ChildGuard::spawn();
    let pid = child.id();

    terminate_process_tree(pid, true).unwrap();

    assert!(!child.wait().success());
    assert!(!is_process_alive(pid));
}

#[test]
fn resilience_termination_propagates_capture_failure_for_a_live_root() {
    let mut child = ChildGuard::spawn();
    let failures = std::iter::repeat_with(|| Err(io::Error::other(INVENTORY_FAILURE_DETAIL)))
        .take(PROCESS_INVENTORY_CAPTURE_ATTEMPTS);
    let _inventory = install_test_inventory_results(failures);

    let error = terminate_process_tree(child.id(), false).unwrap_err();

    assert!(format!("{error:#}").contains(INVENTORY_FAILURE_DETAIL));
    child.terminate();
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn spawn() -> Self {
        let child = spawn_detached(
            Path::new(SLEEP_EXECUTABLE),
            &[SLEEP_SECONDS.into()],
            DetachedStandardError::Null,
        )
        .unwrap();
        Self(Some(child))
    }

    fn id(&self) -> u32 {
        self.0.as_ref().unwrap().id()
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        self.0.take().unwrap().wait().unwrap()
    }

    fn terminate(&mut self) {
        let child = self.0.as_mut().unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        self.0 = None;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
