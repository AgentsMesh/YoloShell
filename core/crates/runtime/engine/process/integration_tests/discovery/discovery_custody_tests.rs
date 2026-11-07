use super::{
    ProcessDiscoveryCapability, ProcessDiscoveryCustody, ProcessIdentity,
    install_test_live_signal_failure,
};
use std::ffi::CString;
use std::ptr;
use std::time::Duration;

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_ARGUMENT: &str = "30";
const CAPABILITY_LENGTH: usize = 64;
const GRACEFUL_TIMEOUT: Duration = Duration::from_millis(200);
const FORCE_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const REFRESH_FAILURE: &str = "injected discovery refresh failure";
const RESUME_FAILURE: &str = "injected exact resume failure";
const CHILD_CAPABILITY_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 17;

#[test]
fn capability_is_random_lowercase_hex() {
    let first = ProcessDiscoveryCapability::generate().unwrap();
    let second = ProcessDiscoveryCapability::generate().unwrap();
    assert!(first != second);
    assert_eq!(first.value().len(), CAPABILITY_LENGTH);
    assert!(
        first
            .value()
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
}

#[test]
fn suspended_root_is_resumed_and_settled() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let inheritance = capability.begin_writer_inheritance().unwrap();
    let mut child = SuspendedProcess::spawn(capability.value(), inheritance.descriptor());
    let identity = child.identity();
    let mut custody =
        ProcessDiscoveryCustody::capture_suspended(identity.pid, capability, inheritance).unwrap();
    custody.refresh().unwrap();
    custody
        .settle(GRACEFUL_TIMEOUT, FORCE_TIMEOUT, POLL_INTERVAL)
        .unwrap();
    assert!(!identity.matches());
    child.reap();
}

#[test]
fn refresh_failure_still_closes_custody_and_fails() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let inheritance = capability.begin_writer_inheritance().unwrap();
    let mut child = SuspendedProcess::spawn(capability.value(), inheritance.descriptor());
    let identity = child.identity();
    let mut custody =
        ProcessDiscoveryCustody::capture_suspended(identity.pid, capability, inheritance).unwrap();
    custody.fail_next_refresh_for_test(REFRESH_FAILURE);
    let error = custody
        .settle(GRACEFUL_TIMEOUT, FORCE_TIMEOUT, POLL_INTERVAL)
        .unwrap_err();
    assert!(format!("{error:#}").contains(REFRESH_FAILURE));
    assert!(!identity.matches());
    child.reap();
}

#[test]
fn exact_resume_failure_leaves_suspended_root_for_rollback() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let inheritance = capability.begin_writer_inheritance().unwrap();
    let mut child = SuspendedProcess::spawn(capability.value(), inheritance.descriptor());
    let identity = child.identity();
    let guard = install_test_live_signal_failure(|| Some(RESUME_FAILURE.to_owned()));
    let error = ProcessDiscoveryCustody::capture_suspended(identity.pid, capability, inheritance)
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains(RESUME_FAILURE));
    assert!(identity.matches());
    drop(guard);
    child.reap();
}

struct SuspendedProcess {
    pid: libc::pid_t,
    identity: ProcessIdentity,
    reaped: bool,
}

impl SuspendedProcess {
    fn spawn(capability: &str, descriptor: libc::c_int) -> Self {
        let executable = CString::new(SLEEP_EXECUTABLE).unwrap();
        let argument = CString::new(SLEEP_ARGUMENT).unwrap();
        let environment = CString::new(format!(
            "{}={capability}",
            super::PROCESS_DISCOVERY_CAPABILITY_ENV
        ))
        .unwrap();
        let mut argv = [
            executable.as_ptr().cast_mut(),
            argument.as_ptr().cast_mut(),
            ptr::null_mut(),
        ];
        let mut envp = [environment.as_ptr().cast_mut(), ptr::null_mut()];
        let mut attributes = ptr::null_mut();
        let mut actions = ptr::null_mut();
        assert_eq!(
            unsafe { libc::posix_spawn_file_actions_init(&mut actions) },
            0
        );
        assert_eq!(
            unsafe {
                libc::posix_spawn_file_actions_adddup2(
                    &mut actions,
                    descriptor,
                    CHILD_CAPABILITY_DESCRIPTOR,
                )
            },
            0
        );
        assert_eq!(unsafe { libc::posix_spawnattr_init(&mut attributes) }, 0);
        let flags = libc::POSIX_SPAWN_START_SUSPENDED | libc::POSIX_SPAWN_SETPGROUP;
        assert_eq!(
            unsafe { libc::posix_spawnattr_setflags(&mut attributes, flags as libc::c_short) },
            0
        );
        assert_eq!(
            unsafe { libc::posix_spawnattr_setpgroup(&mut attributes, 0) },
            0
        );
        let mut pid = 0;
        let status = unsafe {
            libc::posix_spawn(
                &mut pid,
                executable.as_ptr(),
                &actions,
                &attributes,
                argv.as_mut_ptr(),
                envp.as_mut_ptr(),
            )
        };
        assert_eq!(
            unsafe { libc::posix_spawn_file_actions_destroy(&mut actions) },
            0
        );
        assert_eq!(unsafe { libc::posix_spawnattr_destroy(&mut attributes) }, 0);
        assert_eq!(status, 0);
        let identity = ProcessIdentity::capture(u32::try_from(pid).unwrap()).unwrap();
        Self {
            pid,
            identity,
            reaped: false,
        }
    }

    fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    fn reap(&mut self) {
        if self.reaped {
            return;
        }
        let _ = self.identity().signal(libc::SIGKILL);
        let mut status = 0;
        loop {
            let result = unsafe { libc::waitpid(self.pid, &mut status, 0) };
            if result == self.pid {
                self.reaped = true;
                return;
            }
            assert_eq!(
                std::io::Error::last_os_error().kind(),
                std::io::ErrorKind::Interrupted
            );
        }
    }
}

impl Drop for SuspendedProcess {
    fn drop(&mut self) {
        self.reap();
    }
}
