use super::constants::*;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use yolo_runtime::paths::SOCKET_ROOT_ENVIRONMENT;
use yolo_runtime::process::PROCESS_DISCOVERY_CAPABILITY_ENV;

#[test]
fn guard_fixture() {
    let Some(mode) = std::env::var_os(FIXTURE_MODE_ENVIRONMENT) else {
        return;
    };
    let mode = mode.to_str().expect(FIXTURE_MODE_DETAIL);
    assert!(matches!(
        mode,
        NORMAL_MODE
            | RUNNING_LEAF_MODE
            | STOPPED_LEAF_MODE
            | FORK_ON_TERM_MODE
            | FORK_BEFORE_FORCE_MODE
            | FORK_AFTER_REFRESH_MODE
    ));
    let socket_root = std::env::var_os(SOCKET_ROOT_ENVIRONMENT).expect(FIXTURE_PATH_DETAIL);
    std::fs::write(
        required_path(FIXTURE_SOCKET_ROOT_ENVIRONMENT),
        socket_root.as_os_str().as_bytes(),
    )
    .unwrap();
    if mode != NORMAL_MODE {
        helper_command(INTERMEDIATE_ROLE).spawn().unwrap();
        wait_for_path(&required_path(FIXTURE_LEAF_READY_ENVIRONMENT));
    }
    std::fs::write(required_path(FIXTURE_READY_ENVIRONMENT), EMPTY_MARKER).unwrap();
    wait_for_path(&required_path(FIXTURE_RELEASE_ENVIRONMENT));
}

#[test]
fn escape_helper() {
    match std::env::var(FIXTURE_HELPER_ROLE_ENVIRONMENT).as_deref() {
        Ok(INTERMEDIATE_ROLE) => run_intermediate(),
        Ok(LEAF_ROLE) => leaf::run_initial(),
        Ok(LATE_LEAF_ROLE) => leaf::run_late(),
        Ok(REFRESH_HOLDER_ROLE) => leaf::run_refresh_holder(),
        _ => return,
    }
    unsafe { libc::_exit(libc::EXIT_SUCCESS) }
}

fn run_intermediate() {
    helper_command(LEAF_ROLE)
        .env(
            FIXTURE_INTERMEDIATE_PID_ENVIRONMENT,
            std::process::id().to_string(),
        )
        .spawn()
        .unwrap();
    if std::env::var(FIXTURE_MODE_ENVIRONMENT).as_deref() == Ok(FORK_AFTER_REFRESH_MODE) {
        wait_for_path(&required_path(
            FIXTURE_HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT,
        ));
    }
}

fn helper_command(role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            ESCAPE_HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(FIXTURE_HELPER_ROLE_ENVIRONMENT, role)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if std::env::var(FIXTURE_MODE_ENVIRONMENT).as_deref() == Ok(FORK_AFTER_REFRESH_MODE)
        && matches!(role, INTERMEDIATE_ROLE | LEAF_ROLE)
    {
        let capability = std::env::var_os(PROCESS_DISCOVERY_CAPABILITY_ENV)
            .or_else(|| std::env::var_os(FIXTURE_DISCOVERY_CAPABILITY_ENVIRONMENT))
            .expect(FIXTURE_PATH_DETAIL);
        command
            .env(FIXTURE_DISCOVERY_CAPABILITY_ENVIRONMENT, capability)
            .env_remove(PROCESS_DISCOVERY_CAPABILITY_ENV);
    }
    command
}

#[path = "../test_support/fixture_leaf.rs"]
mod leaf;

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .expect(FIXTURE_PATH_DETAIL)
}

fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + FIXTURE_LIFETIME;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{FIXTURE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
