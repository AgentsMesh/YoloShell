use super::super::{StartupChildGuard, test_output};
#[cfg(target_os = "macos")]
use super::process_descriptor;
use crate::process::{PTY_SESSION_CAPABILITY_ENV, ProcessIdentity, PtySessionProof};
use crate::worker::constants::{BOOTSTRAP_TEST_EXACT_ARGUMENT, BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use uuid::Uuid;
#[cfg(feature = "yolo-unit-tests")]
pub(super) const TEST_HELPER_ENTRY: &str =
    "worker::startup::startup_guard::tests::process_fixture::startup_guard_process_helper_unit";
#[cfg(feature = "yolo-integration-tests")]
pub(super) const TEST_HELPER_ENTRY: &str = "worker::startup::startup_guard::tests::process_fixture::startup_guard_process_helper_integration";
pub(super) const TEST_HELPER_ROLE_ENV: &str = "YOLO_STARTUP_GUARD_TEST_ROLE";
pub(super) const TEST_HELPER_MARKER_ENV: &str = "YOLO_STARTUP_GUARD_TEST_MARKER";
const TEST_HELPER_RELEASE_ENV: &str = "YOLO_STARTUP_GUARD_TEST_RELEASE";
pub(super) const TEST_ROOT_ROLE: &str = "root";
const TEST_DESCENDANT_ROLE: &str = "descendant";
const TEST_LINEAGE_DESCENDANT_ROLE: &str = "lineage-descendant";
pub(in crate::worker::startup::startup_guard) const TEST_MARKER_FILE: &str = "setsid-descendant";
const TEST_ROOT_EXIT_EXTENSION: &str = "exit";
const TEST_ROOT_RELEASE_EXTENSION: &str = "release";
const TEST_ROOT_SIGNAL_RESISTANCE_ENV: &str = "YOLO_TEST_ROOT_SIGNAL_RESISTANCE";
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const TEST_READY_TIMEOUT: Duration = Duration::from_secs(3);
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TEST_DESCENDANT_TIMEOUT_DETAIL: &str = "descendant was not ready";
const TEST_TRACKING_TIMEOUT_DETAIL: &str = "scope monitor missed descendant";
const INVALID_TEST_HELPER_ROLE_DETAIL: &str = "invalid startup guard helper role";
const SUCCESS_EXIT_CODE: i32 = 0;
pub(super) type TestMaster = Box<dyn MasterPty + Send>;
type TestGuard = (StartupChildGuard, TestMaster, ProcessIdentity);

pub(in crate::worker::startup::startup_guard) struct TestRootRelease(Option<PathBuf>);
impl TestRootRelease {
    pub(super) fn hold(marker: &Path, command: &mut CommandBuilder) -> Self {
        let path = marker.with_extension(TEST_ROOT_RELEASE_EXTENSION);
        command.env(TEST_HELPER_RELEASE_ENV, &path);
        Self(Some(path))
    }

    pub(in crate::worker::startup::startup_guard) fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if let Some(path) = self.0.take() {
            std::fs::write(path, []).unwrap();
        }
    }
}

impl Drop for TestRootRelease {
    fn drop(&mut self) {
        self.release_inner();
    }
}

fn run_startup_guard_process_helper() {
    let Ok(role) = std::env::var(TEST_HELPER_ROLE_ENV) else {
        return;
    };
    let marker = std::env::var_os(TEST_HELPER_MARKER_ENV).unwrap();
    match role.as_str() {
        TEST_ROOT_ROLE => run_root_helper(&marker),
        TEST_DESCENDANT_ROLE => run_descendant_helper(Path::new(&marker), true),
        TEST_LINEAGE_DESCENDANT_ROLE => run_descendant_helper(Path::new(&marker), false),
        _ => panic!("{INVALID_TEST_HELPER_ROLE_DETAIL}"),
    }
}

#[cfg(all(feature = "yolo-unit-tests", feature = "yolo-worker-unit-startup"))]
include!("../../../unit_tests/startup/startup_guard_process_entrypoint.rs");
#[cfg(feature = "yolo-integration-tests")]
include!("../../../integration_tests/startup/startup_guard_process_entrypoint.rs");

include!("startup_guard_tracked_fixture.rs");

pub(super) fn helper_command(marker: &Path, role: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new(std::env::current_exe().unwrap());
    command.args([
        BOOTSTRAP_TEST_EXACT_ARGUMENT,
        TEST_HELPER_ENTRY,
        BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
    ]);
    command.env(TEST_HELPER_ROLE_ENV, role);
    command.env(TEST_HELPER_MARKER_ENV, marker);
    command
}

pub(super) fn configure_signal_resistant_root(command: &mut CommandBuilder) {
    command.env(TEST_ROOT_SIGNAL_RESISTANCE_ENV, true.to_string());
}

fn run_root_helper(marker: &std::ffi::OsStr) -> ! {
    #[cfg(target_os = "macos")]
    let descriptor = process_descriptor::receive_root_writer();
    if let Some(release) = std::env::var_os(TEST_HELPER_RELEASE_ENV) {
        wait_for_path(Path::new(&release));
    }
    if std::env::var_os(TEST_ROOT_SIGNAL_RESISTANCE_ENV).is_some() {
        ignore_signal(libc::SIGHUP);
        ignore_signal(libc::SIGTERM);
    }
    #[cfg(target_os = "macos")]
    let descendant_role = if descriptor.is_some() {
        TEST_DESCENDANT_ROLE
    } else {
        TEST_LINEAGE_DESCENDANT_ROLE
    };
    #[cfg(not(target_os = "macos"))]
    let descendant_role = TEST_LINEAGE_DESCENDANT_ROLE;
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            BOOTSTRAP_TEST_EXACT_ARGUMENT,
            TEST_HELPER_ENTRY,
            BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
        ])
        .env(TEST_HELPER_ROLE_ENV, descendant_role)
        .env(TEST_HELPER_MARKER_ENV, marker);
    #[cfg(target_os = "macos")]
    process_descriptor::configure_descendant(descriptor.as_ref(), &mut command);
    let mut descendant = crate::process::spawn_waitable_test_child(command);
    std::thread::spawn(move || {
        let _ = descendant.wait();
    });
    wait_for_path(&Path::new(marker).with_extension(TEST_ROOT_EXIT_EXTENSION));
    std::process::exit(SUCCESS_EXIT_CODE)
}

fn run_descendant_helper(marker: &Path, _descriptor_required: bool) -> ! {
    #[cfg(target_os = "macos")]
    if _descriptor_required {
        process_descriptor::assert_descendant_writer();
    }
    let pid = std::process::id();
    let session = unsafe { libc::setsid() };
    assert_eq!(session, i32::try_from(pid).unwrap());
    ignore_signal(libc::SIGHUP);
    ignore_signal(libc::SIGTERM);
    std::fs::write(marker, pid.to_string()).unwrap();
    wait_forever()
}

fn ignore_signal(signal: i32) {
    let previous = unsafe { libc::signal(signal, libc::SIG_IGN) };
    assert_ne!(previous, libc::SIG_ERR);
}

fn wait_for_path(path: &Path) {
    while !path.exists() {
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_forever() -> ! {
    loop {
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

include!("startup_guard_process_wait.rs");
