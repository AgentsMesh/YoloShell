use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const HELPER_TEST: &str =
    "captured_tree_discovery_adversarial_tests::fixture::helper::rapid_escape_helper";
const ROLE_ENVIRONMENT: &str = "YOLO_DISCOVERY_HELPER_ROLE";
const ROOT_READY_ENVIRONMENT: &str = "YOLO_DISCOVERY_ROOT_READY";
const TRIGGER_ENVIRONMENT: &str = "YOLO_DISCOVERY_TRIGGER";
const LEAF_PID_ENVIRONMENT: &str = "YOLO_DISCOVERY_LEAF_PID";
const LEAF_READY_ENVIRONMENT: &str = "YOLO_DISCOVERY_LEAF_READY";
const INTERMEDIATE_PID_ENVIRONMENT: &str = "YOLO_DISCOVERY_INTERMEDIATE_PID";
const SCRUB_LEAF_ENVIRONMENT: &str = "YOLO_DISCOVERY_SCRUB_LEAF_ENVIRONMENT";
pub(super) const ROOT_ROLE: &str = "root";
const INTERMEDIATE_ROLE: &str = "intermediate";
const LEAF_ROLE: &str = "leaf";
const ENABLED: &str = "1";
const EMPTY_PATH: &str = "";
const EMPTY_MARKER: &[u8] = b"";
const HELPER_LIFETIME: Duration = Duration::from_secs(30);
pub(super) const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const SYSTEM_CALL_FAILURE: libc::pid_t = -1;
const HELPER_TIMEOUT_DETAIL: &str = "rapid escaped descendant helper timed out";

#[test]
fn rapid_escape_helper() {
    match std::env::var(ROLE_ENVIRONMENT).as_deref() {
        Ok(ROOT_ROLE) => run_root(),
        Ok(INTERMEDIATE_ROLE) => run_intermediate(),
        Ok(LEAF_ROLE) => run_leaf(),
        _ => return,
    }
    unsafe { libc::_exit(libc::EXIT_SUCCESS) }
}

fn run_root() {
    std::fs::write(required_path(ROOT_READY_ENVIRONMENT), EMPTY_MARKER).unwrap();
    wait_for_path(&required_path(TRIGGER_ENVIRONMENT), HELPER_LIFETIME);
    crate::process::spawn_waitable_test_child(&mut command(
        INTERMEDIATE_ROLE,
        Path::new(EMPTY_PATH),
        Path::new(EMPTY_PATH),
        &required_path(LEAF_PID_ENVIRONMENT),
        &required_path(LEAF_READY_ENVIRONMENT),
    ));
}

fn run_intermediate() {
    let mut leaf = command(
        LEAF_ROLE,
        Path::new(EMPTY_PATH),
        Path::new(EMPTY_PATH),
        &required_path(LEAF_PID_ENVIRONMENT),
        &required_path(LEAF_READY_ENVIRONMENT),
    );
    leaf.env(INTERMEDIATE_PID_ENVIRONMENT, std::process::id().to_string());
    if std::env::var(SCRUB_LEAF_ENVIRONMENT).as_deref() == Ok(ENABLED) {
        leaf.env_remove(crate::process::PROCESS_DISCOVERY_CAPABILITY_ENV);
    }
    crate::process::spawn_waitable_test_child(leaf);
}

fn run_leaf() {
    if std::env::var(SCRUB_LEAF_ENVIRONMENT).as_deref() == Ok(ENABLED) {
        assert!(std::env::var(crate::process::PROCESS_DISCOVERY_CAPABILITY_ENV).is_err());
    }
    let intermediate: libc::pid_t = std::env::var(INTERMEDIATE_PID_ENVIRONMENT)
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(unsafe { libc::setsid() }, SYSTEM_CALL_FAILURE);
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while unsafe { libc::getppid() } == intermediate {
        assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
    std::fs::write(
        required_path(LEAF_PID_ENVIRONMENT),
        std::process::id().to_string(),
    )
    .unwrap();
    std::fs::write(required_path(LEAF_READY_ENVIRONMENT), EMPTY_MARKER).unwrap();
    std::thread::sleep(HELPER_LIFETIME);
}

pub(super) fn configure_leaf_environment_scrub(command: &mut Command) {
    command.env(SCRUB_LEAF_ENVIRONMENT, ENABLED);
}

pub(super) fn command(
    role: &str,
    root: &Path,
    trigger: &Path,
    pid: &Path,
    ready: &Path,
) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(ROLE_ENVIRONMENT, role)
        .env(ROOT_READY_ENVIRONMENT, root)
        .env(TRIGGER_ENVIRONMENT, trigger)
        .env(LEAF_PID_ENVIRONMENT, pid)
        .env(LEAF_READY_ENVIRONMENT, ready)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    command
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

fn wait_for_path(path: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
