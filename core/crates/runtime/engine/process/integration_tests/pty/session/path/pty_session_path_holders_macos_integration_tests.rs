use super::super::constants::MAX_PTY_PROOF_CANDIDATES;
use super::macos_path_holders::path_holders;
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use portable_pty::{PtySize, native_pty_system};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const HELPER_TEST: &str =
    "pty_session_proof::macos_path_holder_integration_tests::setsid_noctty_holder_helper";
const HELPER_ROLE_ENV: &str = "YOLO_PATH_HOLDER_HELPER";
const HELPER_PATH_ENV: &str = "YOLO_PATH_HOLDER_TTY";
const HELPER_READY_ENV: &str = "YOLO_PATH_HOLDER_READY";
const HELPER_RELEASE_ENV: &str = "YOLO_PATH_HOLDER_RELEASE";
const HELPER_ROLE: &str = "setsid-noctty";
const HELPER_READY_COMPONENT: &str = "ready";
const HELPER_RELEASE_COMPONENT: &str = "release";
const HELPER_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const EMPTY_MARKER: &[u8] = b"";
const DUPLICATE_CONTEXT: &str = "duplicate PTY slave descriptor";
const HELPER_WAIT_CONTEXT: &str = "wait for path-holder helper";
const HOLDER_APPEAR_TIMEOUT: &str = "path holder did not appear";
const HOLDER_CLOSE_TIMEOUT: &str = "closed path holder remained visible";
const HELPER_READY_TIMEOUT: &str = "path-holder helper did not become ready";
const HELPER_RELEASE_TIMEOUT: &str = "path-holder helper was not released";
const SETSID_CONTEXT: &str = "create path-holder helper session";

#[test]
fn open_and_dup_hold_the_exact_pty_path_until_both_close() {
    let pair = test_pty();
    let path = pair.master.tty_name().unwrap();
    drop(pair.slave);
    let original = open_slave(&path).unwrap();
    let duplicated_fd = unsafe { libc::dup(original.as_raw_fd()) };
    assert!(duplicated_fd >= 0, "{DUPLICATE_CONTEXT}");
    let duplicated = unsafe { File::from_raw_fd(duplicated_fd) };

    assert!(current_holders(&path).contains(&std::process::id()));
    drop(original);
    wait_until(
        || current_holders(&path).contains(&std::process::id()),
        HOLDER_APPEAR_TIMEOUT,
    );
    drop(duplicated);
    wait_until(
        || !current_holders(&path).contains(&std::process::id()),
        HOLDER_CLOSE_TIMEOUT,
    );
}

#[test]
fn setsid_noctty_holder_is_listed_until_its_descriptor_closes() {
    let pair = test_pty();
    let path = pair.master.tty_name().unwrap();
    drop(pair.slave);
    let directory = tempfile::tempdir().unwrap();
    let ready = directory.path().join(HELPER_READY_COMPONENT);
    let release = directory.path().join(HELPER_RELEASE_COMPONENT);
    let mut helper = spawn_helper(&path, &ready, &release);
    wait_until(|| ready.exists(), HELPER_READY_TIMEOUT);
    let pid = helper.child.id();
    wait_until(
        || current_holders(&path).contains(&pid),
        HOLDER_APPEAR_TIMEOUT,
    );

    std::fs::write(&release, EMPTY_MARKER).unwrap();
    helper.wait();
    wait_until(
        || !current_holders(&path).contains(&pid),
        HOLDER_CLOSE_TIMEOUT,
    );
}

#[test]
fn setsid_noctty_holder_helper() {
    if std::env::var(HELPER_ROLE_ENV).ok().as_deref() != Some(HELPER_ROLE) {
        return;
    }
    let session = unsafe { libc::setsid() };
    assert!(
        session >= 0,
        "{SETSID_CONTEXT}: {}",
        io::Error::last_os_error()
    );
    let path = PathBuf::from(std::env::var_os(HELPER_PATH_ENV).unwrap());
    let _holder = open_slave(&path).unwrap();
    let ready = PathBuf::from(std::env::var_os(HELPER_READY_ENV).unwrap());
    std::fs::write(ready, EMPTY_MARKER).unwrap();
    let release = PathBuf::from(std::env::var_os(HELPER_RELEASE_ENV).unwrap());
    wait_until(|| release.exists(), HELPER_RELEASE_TIMEOUT);
}

fn test_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap()
}

fn open_slave(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(path)
}

fn current_holders(path: &Path) -> std::collections::HashSet<u32> {
    path_holders(path, MAX_PTY_PROOF_CANDIDATES).unwrap()
}

fn spawn_helper(path: &Path, ready: &Path, release: &Path) -> HelperChild {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(HELPER_ROLE_ENV, HELPER_ROLE)
        .env(HELPER_PATH_ENV, path)
        .env(HELPER_READY_ENV, ready)
        .env(HELPER_RELEASE_ENV, release)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = crate::process::spawn_waitable_test_child(command);
    HelperChild {
        child,
        release: release.to_owned(),
        waited: false,
    }
}

fn wait_until(mut condition: impl FnMut() -> bool, detail: &str) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !condition() {
        assert!(Instant::now() < deadline, "{detail}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

struct HelperChild {
    child: Child,
    release: PathBuf,
    waited: bool,
}

impl HelperChild {
    fn wait(&mut self) {
        assert!(self.child.wait().expect(HELPER_WAIT_CONTEXT).success());
        self.waited = true;
    }
}

impl Drop for HelperChild {
    fn drop(&mut self) {
        if !self.waited {
            let _ = std::fs::write(&self.release, EMPTY_MARKER);
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
