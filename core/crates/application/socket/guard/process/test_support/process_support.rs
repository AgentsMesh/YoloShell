use super::constants::*;
use super::guard_process::{FailureAction, LeafState};
use crate::fault_contract::{EXIT_MODE, FAILURE_MODE_ENVIRONMENT, PANIC_MODE};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;
use yolo_process_test_support::{ProcessIdentity, observe_process_tree};
const FILESYSTEM_ROOT: &str = std::path::MAIN_SEPARATOR_STR;
pub(super) struct TestLocation {
    pub(super) source: PathBuf,
    pub(super) workspace: OsString,
    pub(super) executable: PathBuf,
}
pub(super) fn test_location() -> TestLocation {
    let source = std::env::var_os(TEST_SOURCE_DIRECTORY_ENVIRONMENT)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(FILESYSTEM_ROOT));
    let workspace = std::env::var_os(TEST_WORKSPACE_ENVIRONMENT).unwrap_or_default();
    let executable = std::env::var_os(FIXTURE_EXECUTABLE_ENVIRONMENT)
        .map(PathBuf::from)
        .unwrap_or_else(|| local_test_executable(&source, &workspace));
    assert!(valid_relative(&executable));
    TestLocation {
        source,
        workspace,
        executable,
    }
}

fn local_test_executable(source: &Path, workspace: &std::ffi::OsStr) -> PathBuf {
    let base = source.join(workspace).canonicalize().unwrap();
    let current = std::env::current_exe().unwrap().canonicalize().unwrap();
    current.strip_prefix(base).unwrap().to_path_buf()
}

fn valid_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub(super) fn guard_binary(location: &TestLocation) -> PathBuf {
    if let Some(value) = std::env::var_os(GUARD_BINARY_ENVIRONMENT) {
        let path = PathBuf::from(value);
        return if path.is_absolute() {
            path
        } else {
            location.source.join(&location.workspace).join(path)
        };
    }
    let current = std::env::current_exe().expect(GUARD_BINARY_DETAIL);
    let dependencies = current
        .parent()
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name == LOCAL_TEST_DEPENDENCY_DIRECTORY)
        })
        .expect(GUARD_BINARY_DETAIL);
    dependencies
        .parent()
        .expect(GUARD_BINARY_DETAIL)
        .join(LOCAL_GUARD_BINARY_FILE)
}

pub(super) fn guard_command(
    location: &TestLocation,
    leaf: LeafState,
    failure: FailureAction,
    ready: &Path,
    release: &Path,
) -> Command {
    let mode = match leaf {
        LeafState::None => NORMAL_MODE,
        LeafState::Running => RUNNING_LEAF_MODE,
        LeafState::Stopped => STOPPED_LEAF_MODE,
        LeafState::ForkOnTerm => FORK_ON_TERM_MODE,
        LeafState::ForkBeforeForce => FORK_BEFORE_FORCE_MODE,
        LeafState::ForkAfterRefresh => FORK_AFTER_REFRESH_MODE,
    };
    let mut command = Command::new(guard_binary(location));
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            FIXTURE_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(TEST_SOURCE_DIRECTORY_ENVIRONMENT, &location.source)
        .env(TEST_WORKSPACE_ENVIRONMENT, &location.workspace)
        .env(TEST_EXECUTABLE_ENVIRONMENT, &location.executable)
        .env(FIXTURE_MODE_ENVIRONMENT, mode)
        .env(FIXTURE_READY_ENVIRONMENT, ready)
        .env(FIXTURE_RELEASE_ENVIRONMENT, release)
        .env_remove(TEST_FILTER_ENVIRONMENT)
        .env_remove(FAILURE_MODE_ENVIRONMENT)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(mode) = match failure {
        FailureAction::Exit => Some(EXIT_MODE),
        FailureAction::Panic => Some(PANIC_MODE),
        FailureAction::None | FailureAction::GuardKill | FailureAction::WatchdogKill => None,
    } {
        command.env(FAILURE_MODE_ENVIRONMENT, mode);
    }
    command
}

pub(super) fn capture_identity(path: &Path) -> ProcessIdentity {
    try_capture_identity(path).expect(FIXTURE_LEAF_DETAIL)
}

pub(super) fn try_capture_identity(path: &Path) -> Option<ProcessIdentity> {
    crate::process_identity_record::read(path).filter(|identity| identity.matches())
}

pub(super) fn capture_child_identity(child: &mut Child) -> ProcessIdentity {
    match ProcessIdentity::capture(child.id()) {
        Ok(identity) => identity,
        Err(error) => {
            let _ = ::std::process::Child::kill(child);
            let deadline = Instant::now() + FALLBACK_CLEANUP_TIMEOUT;
            while Instant::now() < deadline {
                match child.try_wait() {
                    Ok(Some(_)) | Err(_) => break,
                    Ok(None) => std::thread::sleep(POLL_INTERVAL),
                }
            }
            panic!("{FIXTURE_SPAWN_DETAIL}: {error:#}");
        }
    }
}

pub(super) fn capture_watchdog(
    guard: ProcessIdentity,
    leaf: Option<ProcessIdentity>,
) -> ProcessIdentity {
    try_capture_watchdog(guard, leaf).expect(WATCHDOG_IDENTITY_DETAIL)
}

pub(super) fn try_capture_watchdog(
    guard: ProcessIdentity,
    leaf: Option<ProcessIdentity>,
) -> Option<ProcessIdentity> {
    let identities = observe_process_tree(guard).ok()?;
    if let Some(leaf) = leaf {
        (!identities.contains(&leaf)).then_some(())?;
    }
    (exact_session(guard) == Some(guard.pid)).then_some(())?;
    (exact_group(guard) == Some(guard.pid)).then_some(())?;
    let leaders = identities
        .into_iter()
        .filter(|identity| *identity != guard)
        .filter(|identity| exact_session(*identity) == Some(identity.pid))
        .collect::<Vec<_>>();
    (leaders.len() == EXPECTED_SESSION_LEADERS).then_some(())?;
    let watchdog = *leaders.first()?;
    (exact_group(watchdog) == Some(watchdog.pid)).then_some(())?;
    (watchdog.pid != guard.pid).then_some(watchdog)
}

pub(super) fn kill_guard(identity: ProcessIdentity) -> anyhow::Result<()> {
    anyhow::ensure!(identity.signal(libc::SIGKILL)?, GUARD_KILL_DETAIL);
    Ok(())
}
fn exact_session(identity: ProcessIdentity) -> Option<u32> {
    exact_process_value(identity, |pid| unsafe { libc::getsid(pid) })
}

fn exact_group(identity: ProcessIdentity) -> Option<u32> {
    exact_process_value(identity, |pid| unsafe { libc::getpgid(pid) })
}

fn exact_process_value(
    identity: ProcessIdentity,
    read: impl FnOnce(libc::pid_t) -> libc::pid_t,
) -> Option<u32> {
    if !identity.matches() {
        return None;
    }
    let pid = libc::pid_t::try_from(identity.pid).ok()?;
    let value = read(pid);
    (value != SYSTEM_CALL_FAILURE && identity.matches()).then(|| u32::try_from(value).unwrap())
}
