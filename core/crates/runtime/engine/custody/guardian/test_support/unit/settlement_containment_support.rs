use super::*;
use crate::custody_guardian::test_child_guard::GuardedChild;

const TEST_PROCESS: &str = "/bin/sleep";
const TEST_PROCESS_DURATION: &str = "30";
const TEST_SHELL: &str = "/bin/sh";
const TEST_SHELL_COMMAND_ARGUMENT: &str = "-c";
const STOPPED_TERM_IGNORING_SCRIPT: &str = "trap '' TERM; kill -STOP $$; exit";
const PROCESS_EXIT_TIMEOUT: std::time::Duration =
    crate::session_timing::FORCE_TERMINATION_CONFIRM_TIMEOUT;
const PROCESS_POLL_INTERVAL: std::time::Duration =
    crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const PROCESS_EXIT_TIMEOUT_DETAIL: &str = "test containment process did not exit";
const PROCESS_STOP_TIMEOUT_DETAIL: &str = "test containment process did not stop";
const PROCESS_STOP_STATUS_DETAIL: &str = "test containment process reported a non-stop event";
const WAITPID_FAILURE: libc::pid_t = -1;
const WAITPID_NO_EVENT: libc::pid_t = 0;

pub(super) fn runtime(worker: ProcessIdentity, child: ProcessIdentity) -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: worker.pid,
        worker_start_time: worker.start_time,
        child_pid: child.pid,
        child_start_time: child.start_time,
        boot_id: crate::process::boot_id(),
    }
}

pub(super) struct ChildGuard {
    child: GuardedChild,
    pub(super) identity: ProcessIdentity,
}

impl ChildGuard {
    pub(super) fn spawn() -> Self {
        let mut command = std::process::Command::new(TEST_PROCESS);
        command.arg(TEST_PROCESS_DURATION);
        Self::spawn_command(command)
    }

    pub(super) fn spawn_stopped_term_ignoring() -> Self {
        let mut command = std::process::Command::new(TEST_SHELL);
        command
            .arg(TEST_SHELL_COMMAND_ARGUMENT)
            .arg(STOPPED_TERM_IGNORING_SCRIPT);
        let mut child = Self::spawn_command(command);
        child.wait_for_stop();
        child
    }

    fn spawn_command(command: std::process::Command) -> Self {
        let child = GuardedChild::spawn(move || crate::process::spawn_waitable_test_child(command));
        let identity = child.identity();
        Self { child, identity }
    }

    pub(super) fn wait_for_exit(&mut self) -> std::process::ExitStatus {
        let deadline = std::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "{PROCESS_EXIT_TIMEOUT_DETAIL}"
            );
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
    }

    fn wait_for_stop(&mut self) {
        let deadline = std::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
        let mut status = libc::c_int::default();
        let pid = self.identity.pid.try_into().unwrap();
        loop {
            let observed =
                unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG | libc::WUNTRACED) };
            assert_ne!(observed, WAITPID_FAILURE, "{PROCESS_STOP_STATUS_DETAIL}");
            if observed == pid {
                assert!(libc::WIFSTOPPED(status), "{PROCESS_STOP_STATUS_DETAIL}");
                return;
            }
            assert_eq!(observed, WAITPID_NO_EVENT, "{PROCESS_STOP_STATUS_DETAIL}");
            assert!(
                std::time::Instant::now() < deadline,
                "{PROCESS_STOP_TIMEOUT_DETAIL}"
            );
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
    }
}
