use super::*;
use crate::process::ProcessIdentity;
use std::io;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const STOP_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(10);
const GENERATION_OFFSET: u64 = 1;
const FIRST_TASK: u32 = 101;
const SECOND_TASK: u32 = 102;
const RUNNING_STATE: u8 = b'S';
const STOP_TIMEOUT_DETAIL: &str = "retained Linux process did not reach stopped state";

struct ChildGuard {
    child: Option<Child>,
    control: crate::process::LiveProcessIdentity,
    identity: ProcessIdentity,
    retained: RetainedPidfd,
}

impl ChildGuard {
    fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(SLEEP_EXECUTABLE).arg(SLEEP_DURATION),
        );
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let retained = RetainedPidfd::capture(identity).unwrap().unwrap();
        let control = crate::process::LiveProcessIdentity::capture(identity)
            .unwrap()
            .unwrap();
        Self {
            child: Some(child),
            control,
            identity,
            retained,
        }
    }

    fn signal(&self, signal: i32) {
        assert!(self.control.signal_for_test(signal).unwrap());
    }

    fn kill_and_reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            self.signal(libc::SIGKILL);
            child.wait().unwrap();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.child.is_some() {
            self.signal(libc::SIGCONT);
            self.kill_and_reap();
        }
    }
}

#[test]
fn retained_pidfd_observes_running_then_all_threads_stopped() {
    let child = ChildGuard::spawn();
    assert_eq!(observed(&child), ProcessExecutionState::Running);

    child.signal(libc::SIGSTOP);
    let deadline = Instant::now() + STOP_TIMEOUT;
    while observed(&child) != ProcessExecutionState::Stopped {
        assert!(Instant::now() < deadline, "{STOP_TIMEOUT_DETAIL}");
        std::thread::sleep(STOP_POLL_INTERVAL);
    }
}

#[test]
fn retained_authority_rejects_a_different_generation_as_gone() {
    let child = ChildGuard::spawn();
    let generation = child.identity.start_time.wrapping_add(GENERATION_OFFSET);

    assert_eq!(
        execution_state(&child.retained, child.identity.pid, generation).unwrap(),
        ProcessExecutionState::Gone
    );
    assert!(child.identity.matches());
}

#[test]
fn missing_procfs_is_error_while_live_but_gone_after_pidfd_exit() {
    let mut child = ChildGuard::spawn();
    let live_error = classify_missing_procfs(&child.retained, missing_procfs()).unwrap_err();
    assert_eq!(
        live_error
            .downcast_ref::<io::Error>()
            .and_then(io::Error::raw_os_error),
        Some(libc::ENOENT)
    );

    child.kill_and_reap();
    assert_eq!(
        classify_missing_procfs(&child.retained, missing_procfs()).unwrap(),
        ProcessExecutionState::Gone
    );
}

#[test]
fn task_snapshots_require_identical_all_stopped_states() {
    let stopped = BTreeSet::from([
        (FIRST_TASK, STOPPED_JOB_CONTROL_STATE),
        (SECOND_TASK, STOPPED_TRACED_STATE),
    ]);
    assert_eq!(
        classify_task_states(&stopped, &stopped),
        ProcessExecutionState::Stopped
    );

    let changed = BTreeSet::from([
        (FIRST_TASK, STOPPED_JOB_CONTROL_STATE),
        (SECOND_TASK, RUNNING_STATE),
    ]);
    assert_eq!(
        classify_task_states(&stopped, &changed),
        ProcessExecutionState::Running
    );
    assert_eq!(
        classify_task_states(&changed, &changed),
        ProcessExecutionState::Running
    );
}

fn observed(child: &ChildGuard) -> ProcessExecutionState {
    execution_state(
        &child.retained,
        child.identity.pid,
        child.identity.start_time,
    )
    .unwrap()
}

fn missing_procfs() -> anyhow::Error {
    io::Error::from_raw_os_error(libc::ENOENT).into()
}
