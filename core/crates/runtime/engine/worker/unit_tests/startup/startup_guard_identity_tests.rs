use super::*;
use crate::process::{DetachedStandardError, spawn_detached};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_SLEEP_DURATION: &str = "30";
const TEST_EXIT_CODE: u32 = 0;
const INCOMPLETE_IDENTITY_PID: u32 = 42;
const INCOMPLETE_IDENTITY_START_TIME: u64 = 84;
const EXPECTED_KILL_COUNT: usize = 1;
const FAIL_STOP_HELPER_ENVIRONMENT: &str = "YOLO_STARTUP_SPAWN_FAIL_STOP_HELPER";
const FAIL_STOP_HELPER_VALUE: &str = "enabled";
const FAIL_STOP_HELPER_FILTER: &str =
    "worker::startup::startup_guard::identity_tests::spawn_boundary_fail_stop_subprocess_helper";

#[derive(Clone, Copy, Debug)]
enum SpawnWaitOutcome {
    Exited,
    Failed,
}

#[derive(Debug)]
struct UnidentifiedSpawnChild {
    kills: Arc<AtomicUsize>,
    wait: SpawnWaitOutcome,
}

impl portable_pty::ChildKiller for UnidentifiedSpawnChild {
    fn kill(&mut self) -> io::Result<()> {
        self.kills.fetch_add(EXPECTED_KILL_COUNT, Ordering::AcqRel);
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self {
            kills: Arc::clone(&self.kills),
            wait: self.wait,
        })
    }
}

impl portable_pty::Child for UnidentifiedSpawnChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        match self.wait {
            SpawnWaitOutcome::Exited if self.kills.load(Ordering::Acquire) > 0 => Ok(Some(
                portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE),
            )),
            SpawnWaitOutcome::Exited => Ok(None),
            SpawnWaitOutcome::Failed => Err(io::ErrorKind::BrokenPipe.into()),
        }
    }

    fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        match self.wait {
            SpawnWaitOutcome::Exited => {
                Ok(portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE))
            }
            SpawnWaitOutcome::Failed => Err(io::ErrorKind::BrokenPipe.into()),
        }
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}

#[test]
fn destructive_cleanup_skips_a_stale_process_identity() {
    let mut child = spawn_detached(
        Path::new(TEST_SLEEP_EXECUTABLE),
        &[TEST_SLEEP_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let stale = ProcessIdentity {
        start_time: identity.start_time.wrapping_add(1),
        ..identity
    };
    let result = stale.signal(libc::SIGKILL);
    let survived = identity.matches();
    let _ = identity.signal(libc::SIGKILL);
    let _ = child.wait();
    assert!(!result.unwrap());
    assert!(survived);
}

#[test]
fn spawn_boundary_settles_child_when_identity_capture_is_impossible() {
    let kills = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::new(Box::new(UnidentifiedSpawnChild {
        kills: Arc::clone(&kills),
        wait: SpawnWaitOutcome::Exited,
    }));
    let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();

    guard.settle_spawn_without_identity(&spawn_guard.boundary());

    assert!(guard.untracked_gone_committed);
    assert_eq!(kills.load(Ordering::Acquire), EXPECTED_KILL_COUNT);
    guard.clear();
}

#[test]
fn incomplete_identity_moves_into_background_cleanup_custody() {
    let kills = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::new(Box::new(UnidentifiedSpawnChild {
        kills,
        wait: SpawnWaitOutcome::Exited,
    }));
    let durable = ProcessIdentity {
        pid: INCOMPLETE_IDENTITY_PID,
        start_time: INCOMPLETE_IDENTITY_START_TIME,
    };
    guard.identity = Some(durable);
    guard.live_identity = None;

    let identity = guard.take_untracked_identity();

    assert!(matches!(
        identity,
        crate::worker::startup::startup_untracked_reaper::StartupUntrackedIdentity::Incomplete {
            durable: Some(observed),
            live: None,
        } if observed == durable
    ));
    assert!(guard.identity.is_none());
    assert!(guard.live_identity.is_none());
    guard.clear();
}

#[test]
fn spawn_boundary_terminal_failure_fail_stops_worker() {
    let status = Command::new(std::env::current_exe().unwrap())
        .arg(FAIL_STOP_HELPER_FILTER)
        .arg(crate::worker::constants::BOOTSTRAP_TEST_EXACT_ARGUMENT)
        .arg(crate::worker::constants::BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT)
        .env(FAIL_STOP_HELPER_ENVIRONMENT, FAIL_STOP_HELPER_VALUE)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(libc::EXIT_FAILURE));
}

#[test]
fn spawn_boundary_fail_stop_subprocess_helper() {
    if std::env::var(FAIL_STOP_HELPER_ENVIRONMENT).as_deref() != Ok(FAIL_STOP_HELPER_VALUE) {
        return;
    }
    let kills = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::new(Box::new(UnidentifiedSpawnChild {
        kills,
        wait: SpawnWaitOutcome::Failed,
    }));
    let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    guard.settle_spawn_without_identity(&spawn_guard.boundary());
}
