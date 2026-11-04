use crate::process::{LiveProcessIdentity, ProcessIdentity, ScopedWaitableChild};
use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) const HELPER_ROLE_ENV: &str = "YOLO_SETEXEC_EXPERIMENT_ROLE";
const HELPER_TARGET_ENV: &str = "YOLO_SETEXEC_EXPERIMENT_TARGET";
const HELPER_RELEASE_ENV: &str = "YOLO_SETEXEC_EXPERIMENT_RELEASE";
pub(super) const HELPER_ROLE: &str = "suspended-setexec";
const HELPER_TEST: &str = "macos_mapped_executable::process_tests::suspended_setexec_helper";
pub(super) const TARGET_FILE: &str = "provider-executable";
pub(super) const REPLACEMENT_FILE: &str = "provider-replacement";
pub(super) const RELEASE_FILE: &str = "setexec.release";
pub(super) const ORIGINAL_EXECUTABLE: &str = "/usr/bin/true";
pub(super) const REPLACEMENT_EXECUTABLE: &str = "/usr/bin/false";
const PRIVATE_EXECUTABLE_MODE: u32 = 0o500;
pub(super) const SPAWN_SUCCESS: libc::c_int = 0;
pub(super) const EMPTY_SIGNAL: &[u8] = &[];
pub(super) const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_POLL: Duration = Duration::from_millis(5);

pub(super) fn wait_for_release_and_target() -> PathBuf {
    let release = PathBuf::from(std::env::var_os(HELPER_RELEASE_ENV).unwrap());
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while !release.try_exists().unwrap() {
        assert!(
            Instant::now() < deadline,
            "SETEXEC helper release timed out"
        );
        std::thread::sleep(WAIT_POLL);
    }
    PathBuf::from(std::env::var_os(HELPER_TARGET_ENV).unwrap())
}

pub(super) fn overlay_suspended(target: &Path) -> libc::c_int {
    let target = CString::new(target.as_os_str().as_bytes()).unwrap();
    let mut arguments = [target.as_ptr().cast_mut(), std::ptr::null_mut()];
    let mut environment = [std::ptr::null_mut()];
    let mut attributes = std::ptr::null_mut();
    assert_eq!(
        unsafe { libc::posix_spawnattr_init(&mut attributes) },
        SPAWN_SUCCESS
    );
    let flags = libc::POSIX_SPAWN_SETEXEC | libc::POSIX_SPAWN_START_SUSPENDED;
    assert_eq!(
        unsafe { libc::posix_spawnattr_setflags(&mut attributes, flags as libc::c_short) },
        SPAWN_SUCCESS
    );
    let mut pid = libc::pid_t::default();
    let status = unsafe {
        libc::posix_spawn(
            &mut pid,
            target.as_ptr(),
            std::ptr::null(),
            &attributes,
            arguments.as_mut_ptr(),
            environment.as_mut_ptr(),
        )
    };
    assert_eq!(
        unsafe { libc::posix_spawnattr_destroy(&mut attributes) },
        SPAWN_SUCCESS
    );
    status
}

pub(super) fn copy_executable(source: &Path, target: &Path) {
    std::fs::copy(source, target).unwrap();
    std::fs::set_permissions(
        target,
        std::fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE),
    )
    .unwrap();
}

pub(super) fn spawn_child(target: &Path, release: &Path) -> ScopedWaitableChild {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(HELPER_ROLE_ENV, HELPER_ROLE)
        .env(HELPER_TARGET_ENV, target)
        .env(HELPER_RELEASE_ENV, release)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    ScopedWaitableChild::spawn_before_deadline(&mut command, Instant::now() + WAIT_TIMEOUT).unwrap()
}

pub(super) fn capture_live_identity(pid: u32) -> LiveProcessIdentity {
    let durable = ProcessIdentity::capture(pid).unwrap();
    LiveProcessIdentity::capture(durable).unwrap().unwrap()
}

pub(super) fn wait_for_clean_exit(child: ScopedWaitableChild) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    runtime
        .block_on(child.wait_for_clean_exit(Instant::now() + WAIT_TIMEOUT))
        .unwrap();
}
