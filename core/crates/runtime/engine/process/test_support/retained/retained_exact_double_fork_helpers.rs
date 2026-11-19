use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use portable_pty::CommandBuilder;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const HELPER_TEST: &str = "retained_exact_double_fork_support::double_fork_helper";
const ROLE_ENV: &str = "YOLO_DOUBLE_FORK_ROLE";
const TRIGGER_ENV: &str = "YOLO_DOUBLE_FORK_TRIGGER";
const LEAF_READY_ENV: &str = "YOLO_DOUBLE_FORK_LEAF_READY";
const LEAF_PID_ENV: &str = "YOLO_DOUBLE_FORK_LEAF_PID";
const ROOT_EXIT_ENV: &str = "YOLO_DOUBLE_FORK_ROOT_EXIT";
const CLOAK_ENV: &str = "YOLO_DOUBLE_FORK_CLOAK";
const ROOT_ROLE: &str = "root";
const INTERMEDIATE_ROLE: &str = "intermediate";
const LEAF_ROLE: &str = "leaf";
const ENABLED: &str = "1";
const DISABLED: &str = "0";
const LEAF_LIFETIME: std::time::Duration = std::time::Duration::from_secs(30);
const HELPER_POLL: std::time::Duration = std::time::Duration::from_millis(10);
const EMPTY_MARKER: &[u8] = b"";
const SETSID_FAILURE: libc::pid_t = -1;

pub(super) fn root_command(
    trigger: &Path,
    ready: &Path,
    leaf_pid: &Path,
    exit: &Path,
    cloak: bool,
) -> CommandBuilder {
    let mut command = CommandBuilder::new(std::env::current_exe().unwrap());
    command.args([
        TEST_HARNESS_EXACT_ARGUMENT,
        HELPER_TEST,
        TEST_HARNESS_NOCAPTURE_ARGUMENT,
    ]);
    command.env(ROLE_ENV, ROOT_ROLE);
    command.env(TRIGGER_ENV, trigger);
    command.env(LEAF_READY_ENV, ready);
    command.env(LEAF_PID_ENV, leaf_pid);
    command.env(ROOT_EXIT_ENV, exit);
    command.env(CLOAK_ENV, if cloak { ENABLED } else { DISABLED });
    command
}

pub(super) fn run_role() {
    match std::env::var(ROLE_ENV).as_deref() {
        Ok(ROOT_ROLE) => run_root(),
        Ok(INTERMEDIATE_ROLE) => run_intermediate(),
        Ok(LEAF_ROLE) => run_leaf(),
        _ => {}
    }
}

fn run_root() {
    wait_for(&required_path(TRIGGER_ENV));
    let mut intermediate = spawn_helper(INTERMEDIATE_ROLE);
    intermediate.wait().unwrap();
    wait_for(&required_path(ROOT_EXIT_ENV));
}

fn run_intermediate() {
    let leaf = spawn_helper(LEAF_ROLE);
    std::mem::forget(leaf);
    unsafe { libc::_exit(libc::EXIT_SUCCESS) };
}

fn run_leaf() {
    assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
    if std::env::var(CLOAK_ENV).as_deref() == Ok(ENABLED) {
        let capability =
            std::ffi::CString::new(super::super::constants::PTY_SESSION_CAPABILITY_ENV).unwrap();
        unsafe { libc::unsetenv(capability.as_ptr()) };
        for descriptor in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
            unsafe { libc::close(descriptor) };
        }
    }
    std::fs::write(required_path(LEAF_PID_ENV), std::process::id().to_string()).unwrap();
    std::fs::write(required_path(LEAF_READY_ENV), EMPTY_MARKER).unwrap();
    std::thread::sleep(LEAF_LIFETIME);
}

fn spawn_helper(role: &str) -> Child {
    crate::process::spawn_waitable_test_child(
        Command::new(std::env::current_exe().unwrap())
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                HELPER_TEST,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
            ])
            .env(ROLE_ENV, role),
    )
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

fn wait_for(path: &Path) {
    while !path.exists() {
        std::thread::sleep(HELPER_POLL);
    }
}
