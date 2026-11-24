use super::super::signal_hook::install_pre_signal_hook;
use super::*;
use crate::process::ProcessIdentity;
use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use std::path::Path;
use std::time::{Duration, Instant};

const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_SLEEP_DURATION: &str = "30";
const STALE_PID_VERSION_DELTA: u32 = 1;
const EXEC_RACE_HELPER_TEST: &str = "macos_signal::tests::exec_race_helper";
const EXEC_RACE_ROLE_ENV: &str = "YOLO_MACOS_SIGNAL_TEST_ROLE";
const EXEC_RACE_MARKER_ENV: &str = "YOLO_MACOS_SIGNAL_TEST_MARKER";
const EXEC_RACE_ROLE: &str = "exec-race-helper";
const EXEC_RACE_MARKER: &str = "exec-ready.marker";
const EXEC_RACE_MARKER_CONTENT: &[u8] = b"";
const EXEC_RACE_EXEC_FAILURE: &str = "exec signal-race helper";
const EXEC_RACE_TIMEOUT_FAILURE: &str = "exec did not change pidversion";
const EXEC_RACE_UNIQUE_ID_FAILURE: &str = "exec changed process uniqueid";
const EXEC_RACE_TIMEOUT: Duration = Duration::from_secs(5);
const EXEC_RACE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const STALE_THREAD_ERROR: i32 = libc::EPERM;

struct ChildGuard(Option<std::process::Child>);

impl ChildGuard {
    fn spawn() -> Self {
        Self(Some(crate::process::spawn_waitable_test_child(
            std::process::Command::new(Path::new(TEST_SLEEP_EXECUTABLE)).arg(TEST_SLEEP_DURATION),
        )))
    }

    fn child(&mut self) -> &mut std::process::Child {
        self.0.as_mut().unwrap()
    }

    fn from_child(child: std::process::Child) -> Self {
        Self(Some(child))
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn stale_kernel_pid_version_is_an_atomic_noop() {
    let mut child = ChildGuard::spawn();
    let durable = ProcessIdentity::capture(child.child().id()).unwrap();
    let identity = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let exact = capture_kernel_identity(identity).unwrap().unwrap();
    let stale = KernelProcessIdentity {
        pid_version: exact.pid_version.wrapping_add(STALE_PID_VERSION_DELTA),
        ..exact
    };

    assert!(matches!(
        signal_once(stale, libc::SIGKILL).unwrap(),
        AtomicSignalOutcome::IdentityChanged
    ));
    assert!(durable.matches());
    assert!(matches!(
        signal_once(exact, libc::SIGKILL).unwrap(),
        AtomicSignalOutcome::Delivered
    ));
    child.child().wait().unwrap();
    child.0 = None;
}

#[test]
fn missing_atomic_signal_capability_fails_closed() {
    let mut child = ChildGuard::spawn();
    let durable = ProcessIdentity::capture(child.child().id()).unwrap();
    let identity = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let exact = capture_kernel_identity(identity).unwrap().unwrap();

    assert!(signal_once_with_capability(exact, libc::SIGKILL, None).is_err());
    assert!(durable.matches());
}

#[test]
fn atomic_signal_uses_returned_error_instead_of_thread_errno() {
    let mut child = ChildGuard::spawn();
    let durable = ProcessIdentity::capture(child.child().id()).unwrap();
    let identity = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let exact = capture_kernel_identity(identity).unwrap().unwrap();
    unsafe { *libc::__error() = STALE_THREAD_ERROR };

    assert!(matches!(
        signal_once_with_capability(exact, libc::SIGKILL, Some(identity_changed)),
        Ok(AtomicSignalOutcome::IdentityChanged)
    ));
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(STALE_THREAD_ERROR)
    );
    assert!(durable.matches());
}

#[test]
fn exec_between_kernel_capture_and_signal_refreshes_pid_version() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(EXEC_RACE_MARKER);
    let child = crate::process::spawn_waitable_test_child(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                EXEC_RACE_HELPER_TEST,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
            ])
            .env(EXEC_RACE_ROLE_ENV, EXEC_RACE_ROLE)
            .env(EXEC_RACE_MARKER_ENV, &marker),
    );
    let mut child = ChildGuard::from_child(child);
    let durable = ProcessIdentity::capture(child.child().id()).unwrap();
    let identity = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let initial = capture_kernel_identity(identity).unwrap().unwrap();
    let _hook = install_pre_signal_hook(move || {
        std::fs::write(&marker, EXEC_RACE_MARKER_CONTENT).unwrap();
        wait_for_pid_version_change(identity, initial);
    });

    assert!(identity.signal(libc::SIGKILL).unwrap());
    assert!(!child.child().wait().unwrap().success());
    child.0 = None;
}

#[test]
fn exec_race_helper() {
    if std::env::var(EXEC_RACE_ROLE_ENV).as_deref() != Ok(EXEC_RACE_ROLE) {
        return;
    }
    let marker = std::env::var_os(EXEC_RACE_MARKER_ENV).unwrap();
    while !Path::new(&marker).exists() {
        std::thread::sleep(EXEC_RACE_POLL_INTERVAL);
    }
    use std::os::unix::process::CommandExt;
    let error = std::process::Command::new(TEST_SLEEP_EXECUTABLE)
        .arg(TEST_SLEEP_DURATION)
        .exec();
    panic!("{EXEC_RACE_EXEC_FAILURE}: {error}");
}

fn wait_for_pid_version_change(identity: LiveProcessIdentity, initial: KernelProcessIdentity) {
    let deadline = Instant::now() + EXEC_RACE_TIMEOUT;
    loop {
        if let Some(current) = capture_kernel_identity(identity).unwrap() {
            assert_eq!(
                current.unique_id, initial.unique_id,
                "{EXEC_RACE_UNIQUE_ID_FAILURE}"
            );
            if current.pid_version != initial.pid_version {
                return;
            }
        }
        assert!(Instant::now() < deadline, "{EXEC_RACE_TIMEOUT_FAILURE}");
        std::thread::sleep(EXEC_RACE_POLL_INTERVAL);
    }
}

unsafe extern "C" fn identity_changed(
    _token: *mut AuditToken,
    _signal: libc::c_int,
) -> libc::c_int {
    libc::ESRCH
}
