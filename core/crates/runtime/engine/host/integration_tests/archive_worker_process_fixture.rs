use crate::host::tests::*;
use super::*;

const ARCHIVE_WORKER_FIXTURE_TEST: &str = "host::tests::archive_worker_process_fixture::archive_worker_process_fixture";
const ARCHIVE_WORKER_FIXTURE_ROLE_ENV: &str = "YOLO_ARCHIVE_WORKER_FIXTURE_ROLE";
const ARCHIVE_WORKER_FIXTURE_MODE_ENV: &str = "YOLO_ARCHIVE_WORKER_FIXTURE_MODE";
const ARCHIVE_WORKER_FIXTURE_ROOT_ENV: &str = "YOLO_ARCHIVE_WORKER_FIXTURE_ROOT";
const ARCHIVE_WORKER_FIXTURE_SOCKET_ENV: &str = "YOLO_ARCHIVE_WORKER_FIXTURE_SOCKET";
const ARCHIVE_WORKER_FIXTURE_ROLE: &str = "worker";
const ARCHIVE_WORKER_REPLACEMENT_ROLE: &str = "replacement";
const ARCHIVE_WORKER_READY_HOLD_MODE: &str = "ready-hold";
const ARCHIVE_WORKER_STATUS_DISCONNECT_BOTH_MODE: &str = "status-disconnect-both";
const ARCHIVE_WORKER_EXITED_SHUTDOWN_DROP_BOTH_MODE: &str = "exited-shutdown-drop-both";
const ARCHIVE_WORKER_EXITED_EXIT_BOTH_MODE: &str = "exited-exit-both";
const ARCHIVE_WORKER_EXITED_EXIT_WORKER_MODE: &str = "exited-exit-worker";
const ARCHIVE_WORKER_EXITED_SHUTDOWN_REPLACEMENT_MODE: &str = "exited-shutdown-replacement";
const ARCHIVE_WORKER_RELEASED_SHUTDOWN_EOF_MODE: &str = "released-shutdown-eof";
const ARCHIVE_WORKER_REJECTED_SHUTDOWN_EOF_MODE: &str = "rejected-shutdown-eof";
pub(super) const ARCHIVE_WORKER_RELEASED_SHUTDOWN_REJECTED: &str =
    "released shutdown fixture rejected request";
const ARCHIVE_WORKER_EVIDENCE_FILE: &str = "identity.json";
const ARCHIVE_WORKER_RECORD_FILE: &str = "runtime.json";
const ARCHIVE_WORKER_READY_FILE: &str = "worker.ready";
const ARCHIVE_REPLACEMENT_REQUEST_FILE: &str = "replacement.request";
const ARCHIVE_REPLACEMENT_READY_FILE: &str = "replacement.ready";
const ARCHIVE_REPLACEMENT_RELEASE_FILE: &str = "replacement.release";
const ARCHIVE_WORKER_FIXTURE_MARKER: &[u8] = b"ready";
const ARCHIVE_WORKER_FIXTURE_TIMEOUT: &str = "archive worker fixture timed out";

#[derive(Clone, Copy)]
pub(super) enum ArchiveWorkerFixtureMode {
    ReadyHold,
    StatusDisconnectBoth,
    ExitedThenShutdownDropBoth,
    ExitedThenExitBoth,
    ExitedThenExitWorker,
    ExitedThenShutdownReplacement,
    ReleasedShutdownThenEof,
    RejectedShutdownThenEof,
}

impl ArchiveWorkerFixtureMode {
    fn value(self) -> &'static str {
        match self {
            Self::ReadyHold => ARCHIVE_WORKER_READY_HOLD_MODE,
            Self::StatusDisconnectBoth => ARCHIVE_WORKER_STATUS_DISCONNECT_BOTH_MODE,
            Self::ExitedThenShutdownDropBoth => ARCHIVE_WORKER_EXITED_SHUTDOWN_DROP_BOTH_MODE,
            Self::ExitedThenExitBoth => ARCHIVE_WORKER_EXITED_EXIT_BOTH_MODE,
            Self::ExitedThenExitWorker => ARCHIVE_WORKER_EXITED_EXIT_WORKER_MODE,
            Self::ExitedThenShutdownReplacement => ARCHIVE_WORKER_EXITED_SHUTDOWN_REPLACEMENT_MODE,
            Self::ReleasedShutdownThenEof => ARCHIVE_WORKER_RELEASED_SHUTDOWN_EOF_MODE,
            Self::RejectedShutdownThenEof => ARCHIVE_WORKER_REJECTED_SHUTDOWN_EOF_MODE,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct ArchiveWorkerFixtureEvidence {
    worker_pid: u32,
    worker_start_time: u64,
    child_pid: u32,
    child_start_time: u64,
}

pub(super) struct ArchiveWorkerProcessFixture {
    evidence: ArchiveWorkerFixtureEvidence,
    reaper: Option<std::thread::JoinHandle<std::io::Result<std::process::ExitStatus>>>,
    directory: tempfile::TempDir,
    socket: PathBuf,
}

impl ArchiveWorkerProcessFixture {
    pub(super) fn spawn(socket: PathBuf, mode: ArchiveWorkerFixtureMode) -> Self {
        let directory = tempdir().unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
                ARCHIVE_WORKER_FIXTURE_TEST,
                crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
                crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
            ])
            .env(ARCHIVE_WORKER_FIXTURE_ROLE_ENV, ARCHIVE_WORKER_FIXTURE_ROLE)
            .env(ARCHIVE_WORKER_FIXTURE_MODE_ENV, mode.value())
            .env(ARCHIVE_WORKER_FIXTURE_ROOT_ENV, directory.path())
            .env(ARCHIVE_WORKER_FIXTURE_SOCKET_ENV, &socket)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut process = crate::process::spawn_waitable_test_child(command);
        let worker = process::ProcessIdentity::capture(process.id()).unwrap();
        let reaper = std::thread::spawn(move || process.wait());
        let evidence =
            wait_archive_fixture_json(&directory.path().join(ARCHIVE_WORKER_EVIDENCE_FILE));
        assert_eq!(
            (evidence.worker_pid, evidence.worker_start_time),
            (worker.pid, worker.start_time)
        );
        Self {
            evidence,
            reaper: Some(reaper),
            directory,
            socket,
        }
    }

    pub(super) fn configure(&self, record: &mut RuntimeRecord) {
        record.worker_pid = self.evidence.worker_pid;
        record.worker_start_time = self.evidence.worker_start_time;
        record.child_pid = self.evidence.child_pid;
        record.child_start_time = self.evidence.child_start_time;
        atomic_write_json(&self.path(ARCHIVE_WORKER_RECORD_FILE), record).unwrap();
        wait_archive_fixture_path(&self.path(ARCHIVE_WORKER_READY_FILE));
    }

    pub(super) fn terminate_worker(&mut self) {
        self.signal(self.worker_identity());
        self.reap_worker(false);
        let _ = std::fs::remove_file(&self.socket);
    }

    pub(super) fn terminate_child(&self) {
        self.signal(self.child_identity());
        wait_archive_fixture_identity_exit(self.child_identity());
    }

    pub(super) fn reap_completed_worker(&mut self) {
        self.reap_worker(true);
    }

    pub(super) fn release_replacement(&self) {
        std::fs::write(
            self.path(ARCHIVE_REPLACEMENT_RELEASE_FILE),
            ARCHIVE_WORKER_FIXTURE_MARKER,
        )
        .unwrap();
        wait_archive_fixture_socket_removal(&self.socket);
    }

    fn worker_identity(&self) -> process::ProcessIdentity {
        process::ProcessIdentity {
            pid: self.evidence.worker_pid,
            start_time: self.evidence.worker_start_time,
        }
    }

    pub(super) fn child_identity(&self) -> process::ProcessIdentity {
        process::ProcessIdentity {
            pid: self.evidence.child_pid,
            start_time: self.evidence.child_start_time,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn signal(&self, identity: process::ProcessIdentity) {
        let _ = identity.signal(libc::SIGKILL);
    }

    fn reap_worker(&mut self, require_success: bool) {
        let Some(reaper) = self.reaper.take() else {
            return;
        };
        let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
        while !reaper.is_finished() && std::time::Instant::now() < deadline {
            std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
        }
        let completed = reaper.is_finished();
        if !completed {
            self.signal(self.worker_identity());
        }
        let status = reaper.join().unwrap().unwrap();
        assert!(!require_success || completed && status.success());
        wait_archive_fixture_identity_exit(self.worker_identity());
    }
}

impl Drop for ArchiveWorkerProcessFixture {
    fn drop(&mut self) {
        let _ = std::fs::write(
            self.path(ARCHIVE_REPLACEMENT_RELEASE_FILE),
            ARCHIVE_WORKER_FIXTURE_MARKER,
        );
        self.signal(self.worker_identity());
        self.signal(self.child_identity());
        if let Some(reaper) = self.reaper.take() {
            let _ = reaper.join();
        }
        let _ = std::fs::remove_file(&self.socket);
    }
}

include!("archive_worker_process_fixture_child.rs");
