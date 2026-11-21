use super::*;
use crate::worker::constants::{BOOTSTRAP_TEST_EXACT_ARGUMENT, BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::time::Duration;

const HELPER_RELEASE_ENVIRONMENT: &str = "YOLO_TEST_EXEC_OBSERVER_RELEASE";
const HELPER_ENTRY: &str = "worker::bootstrap::bootstrap_exec_observer::platform::tests::pre_exec_helper";
const HELPER_RELEASE_POLL: Duration = Duration::from_millis(5);
const HELPER_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);
const HELPER_RELEASE_CONTENT: &[u8] = b"release";
const HELPER_RELEASE_FILE: &str = "release";
const TEST_EXEC_PATH: &str = crate::test_executables::TRUE;
const TEST_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const TEST_MISSING_PID: u32 = i32::MAX as u32;

#[test]
fn registration_fails_for_a_missing_process() {
    let target = BootstrapExecTarget {
        process: ProcessIdentity {
            pid: TEST_MISSING_PID,
            start_time: u64::default(),
        },
        thread_id: TEST_MISSING_PID,
    };
    let error = ExecObserver::register(target).err().unwrap();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_EXEC_OBSERVER_REGISTER_CONTEXT)
    );
}

#[test]
fn registration_rejects_a_non_main_exec_thread() {
    let process = ProcessIdentity::capture(std::process::id()).unwrap();
    let target = BootstrapExecTarget {
        process,
        thread_id: TEST_MISSING_PID,
    };

    let error = ExecObserver::register(target).err().unwrap();
    assert!(error.to_string().contains(BOOTSTRAP_EXEC_THREAD_CONTEXT));
}

#[test]
fn observation_times_out_without_exec_or_exit() {
    let observer = ExecObserver::register(BootstrapExecTarget {
        process: ProcessIdentity::capture(std::process::id()).unwrap(),
        thread_id: std::process::id(),
    })
    .unwrap();
    let error = observer.wait_until(Instant::now()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_EXEC_OBSERVER_TIMEOUT_CONTEXT)
    );
}

#[test]
fn observation_accepts_exec_before_immediate_exit() {
    let mut child = PreExecChild::spawn();
    let observer = child.observer();
    child.release();

    observer
        .wait_until(Instant::now() + TEST_WAIT_TIMEOUT)
        .unwrap();
    assert!(child.wait().success());
}

#[test]
fn exit_before_exec_is_rejected() {
    let mut child = PreExecChild::spawn();
    let observer = child.observer();
    child.kill();

    let error = observer
        .wait_until(Instant::now() + TEST_WAIT_TIMEOUT)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT)
    );
    assert!(!child.wait().success());
}

#[test]
fn dropping_an_unreleased_helper_reaps_it_before_removing_coordination() {
    let child = PreExecChild::spawn();
    let coordination = child.coordination.path().to_path_buf();

    drop(child);

    assert!(!coordination.exists());
}

#[test]
fn pre_exec_helper() {
    let Some(release) = std::env::var_os(HELPER_RELEASE_ENVIRONMENT) else {
        return;
    };
    let release = std::path::PathBuf::from(release);
    let deadline = Instant::now() + HELPER_RELEASE_TIMEOUT;
    while !release.try_exists().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(HELPER_RELEASE_POLL);
    }

    let error = Command::new(TEST_EXEC_PATH).exec();
    panic!("exec helper failed: {error}");
}

struct PreExecChild {
    child: Option<Child>,
    coordination: tempfile::TempDir,
}

impl PreExecChild {
    fn spawn() -> Self {
        let coordination = tempfile::tempdir().unwrap();
        let child = crate::process::spawn_waitable_test_child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    BOOTSTRAP_TEST_EXACT_ARGUMENT,
                    HELPER_ENTRY,
                    BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
                ])
                .env(
                    HELPER_RELEASE_ENVIRONMENT,
                    coordination.path().join(HELPER_RELEASE_FILE),
                ),
        );
        Self {
            child: Some(child),
            coordination,
        }
    }

    fn observer(&self) -> ExecObserver {
        let process = ProcessIdentity::capture(self.pid()).unwrap();
        ExecObserver::register(BootstrapExecTarget {
            process,
            thread_id: self.pid(),
        })
        .unwrap()
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().unwrap().id()
    }

    fn release(&mut self) {
        std::fs::write(
            self.coordination.path().join(HELPER_RELEASE_FILE),
            HELPER_RELEASE_CONTENT,
        )
        .unwrap();
    }

    fn kill(&mut self) {
        self.child.as_mut().unwrap().kill().unwrap();
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        let pid = self.pid();
        let status = self.child.as_mut().unwrap().wait().unwrap();
        self.child = None;
        yolo_coverage_test_support::discard_reaped_kill_profile(pid, status).unwrap();
        status
    }
}

impl Drop for PreExecChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            if let Ok(status) = child.wait() {
                yolo_coverage_test_support::discard_reaped_kill_profile(child.id(), status)
                    .unwrap();
            }
        }
    }
}
