use crate::process::constants::{
    PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV, PROCESS_DISCOVERY_CAPABILITY_ENV,
    PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV, PTY_SESSION_CAPABILITY_ENV,
};
use crate::process::{CapturedProcessTree, ProcessIdentity};
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(10);
const WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(10);
const COMPLETION_FILE: &str = "completed";
const SETPGID_SUCCESS: libc::c_int = 0;
const PRIVATE_GROUP_DETAIL: &str = "isolated scan helper did not enter its private process group";
const WATCHDOG_DETAIL: &str = "isolated fork-exec scan helper exceeded its watchdog";
const HELPER_FAILURE_DETAIL: &str = "isolated fork-exec scan helper failed";
const COMPLETION_MISSING_DETAIL: &str = "isolated fork-exec scan helper did not run";

pub(super) fn run_isolated(test: &str, role_name: &str, role: &str, completion_name: &str) {
    let coordination = tempfile::tempdir().unwrap();
    let completion = coordination.path().join(COMPLETION_FILE);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            test,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(role_name, role)
        .env(completion_name, &completion)
        .env_remove(PTY_SESSION_CAPABILITY_ENV)
        .env_remove(PROCESS_DISCOVERY_CAPABILITY_ENV)
        .env_remove(PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV)
        .env_remove(PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    // SAFETY: setpgid is async-signal-safe and does not allocate after fork.
    unsafe {
        command.pre_exec(|| {
            (libc::setpgid(libc::pid_t::default(), libc::pid_t::default()) == SETPGID_SUCCESS)
                .then_some(())
                .ok_or_else(std::io::Error::last_os_error)
        });
    }
    let child = crate::process::spawn_waitable_test_child(command);
    HelperGroup::new(child).wait_success();
    assert!(completion.exists(), "{COMPLETION_MISSING_DETAIL}");
}

struct HelperGroup {
    child: Option<Child>,
    tree: CapturedProcessTree,
}

impl HelperGroup {
    fn new(child: Child) -> Self {
        let group = child.id() as libc::pid_t;
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(identity).unwrap();
        let helper = Self {
            child: Some(child),
            tree,
        };
        // SAFETY: group names the still-running exact child.
        assert_eq!(
            unsafe { libc::getpgid(group) },
            group,
            "{PRIVATE_GROUP_DETAIL}"
        );
        helper
    }

    fn wait_success(mut self) {
        let deadline = Instant::now() + WATCHDOG_TIMEOUT;
        loop {
            self.tree.refresh().unwrap();
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                if !status.success() {
                    self.terminate_tree();
                }
                self.child.take().unwrap().wait().unwrap();
                assert!(status.success(), "{HELPER_FAILURE_DETAIL}: {status}");
                return;
            }
            if Instant::now() >= deadline {
                self.terminate_tree();
                self.child.take().unwrap().wait().unwrap();
                panic!("{WATCHDOG_DETAIL}");
            }
            std::thread::sleep(WATCHDOG_POLL_INTERVAL);
        }
    }

    fn terminate_tree(&mut self) {
        let _ = self.tree.request_force_termination();
    }
}

impl Drop for HelperGroup {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            self.terminate_tree();
            let _ = child.wait();
        }
    }
}
