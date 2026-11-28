use anyhow::{Result, ensure};
use std::time::Duration;
use yolo_runtime::model::{BootstrapDelivery, RuntimeRecord};
use yolo_runtime::paths::{atomic_write_json, read_json};
use yolo_runtime::process::{ProcessIdentity, process_identity_matches};
use yolo_runtime::worker::startup_handoff_test_hook::{
    COMPLETED_MARKER_FILE, COMPLETION_ACK_MARKER_FILE, FAILURE_MARKER_FILE, HANDOFF_MARKER_FILE,
    READY_MARKER_FILE, REAPED_MARKER_FILE, RELEASE_MARKER_FILE, StartupHandoffMarker,
};

#[path = "startup_handoff_process_fixture.rs"]
mod fixture;
#[path = "startup_handoff_process_watchdog.rs"]
mod watchdog;
use fixture::Fixture;

const MARKER_TIMEOUT: Duration = Duration::from_secs(30);
const WORKER_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const WORKER_EXIT_EXPECTED_DETAIL: &str = "failed session worker unexpectedly succeeded";
const CHILD_RELEASE_DETAIL: &str = "released startup child still matches its generation";
const PROCESS_IDENTITY_CHANGE_DETAIL: &str = "startup process identities changed";
const HANDOFF_IDENTITY_CHANGE_DETAIL: &str = "handoff marker changed startup custody";
const REAPED_IDENTITY_CHANGE_DETAIL: &str = "reaped marker changed startup custody";
const COMPLETION_IDENTITY_CHANGE_DETAIL: &str = "completion marker changed startup custody";
const READY_DELIVERY_STATE_DETAIL: &str = "startup ready marker was not pending";
const FAILURE_DELIVERY_STATE_DETAIL: &str = "startup failure marker was not failed";

#[test]
fn session_worker_waits_for_exact_untracked_startup_reap() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut worker = fixture.spawn_worker()?;
    let ready = worker.wait_marker(&fixture.marker(READY_MARKER_FILE), MARKER_TIMEOUT)?;
    ensure!(
        ready.bootstrap_delivery == BootstrapDelivery::Pending,
        READY_DELIVERY_STATE_DETAIL
    );
    worker.observe_child(&ready)?;
    let failure = worker.wait_marker(&fixture.marker(FAILURE_MARKER_FILE), MARKER_TIMEOUT)?;
    ensure!(
        failure.bootstrap_delivery == BootstrapDelivery::Failed,
        FAILURE_DELIVERY_STATE_DETAIL
    );
    ensure!(
        ready.same_processes(&failure),
        PROCESS_IDENTITY_CHANGE_DETAIL
    );

    let record: RuntimeRecord = read_json(&fixture.paths.runtime_record(fixture.terminal_id))?;
    ensure!(record.bootstrap_delivery == BootstrapDelivery::Failed);
    ensure!(failure.matches_record(&record));
    ensure!(!fixture.paths.exit_record(fixture.terminal_id).exists());

    let handoff = worker.wait_marker(&fixture.marker(HANDOFF_MARKER_FILE), MARKER_TIMEOUT)?;
    ensure!(handoff == failure, HANDOFF_IDENTITY_CHANGE_DETAIL);
    worker.assert_running()?;
    ensure!(process_identity_matches(
        failure.child_pid,
        failure.child_start_time
    ));

    atomic_write_json(&fixture.marker(RELEASE_MARKER_FILE), &failure)?;
    let reaped = worker.wait_marker(&fixture.marker(REAPED_MARKER_FILE), MARKER_TIMEOUT)?;
    ensure!(reaped == failure, REAPED_IDENTITY_CHANGE_DETAIL);
    ensure!(
        !process_identity_matches(reaped.child_pid, reaped.child_start_time),
        CHILD_RELEASE_DETAIL
    );
    worker.assert_running()?;
    ensure!(!fixture.paths.exit_record(fixture.terminal_id).exists());

    atomic_write_json(&fixture.marker(COMPLETION_ACK_MARKER_FILE), &failure)?;
    let completed = worker.wait_marker(&fixture.marker(COMPLETED_MARKER_FILE), MARKER_TIMEOUT)?;
    ensure!(completed == failure, COMPLETION_IDENTITY_CHANGE_DETAIL);
    let status = worker.wait_for_exit(WORKER_EXIT_TIMEOUT)?;
    ensure!(!status.success(), WORKER_EXIT_EXPECTED_DETAIL);
    ensure!(!fixture.paths.exit_record(fixture.terminal_id).exists());
    Ok(())
}

trait MarkerExt {
    fn same_processes(&self, other: &Self) -> bool;
    fn matches_record(&self, record: &RuntimeRecord) -> bool;
    fn child_identity(&self) -> ProcessIdentity;
}

impl MarkerExt for StartupHandoffMarker {
    fn same_processes(&self, other: &Self) -> bool {
        self.worker_pid == other.worker_pid
            && self.worker_start_time == other.worker_start_time
            && self.child_pid == other.child_pid
            && self.child_start_time == other.child_start_time
    }

    fn matches_record(&self, record: &RuntimeRecord) -> bool {
        self.runtime_epoch == record.runtime_epoch
            && self.worker_pid == record.worker_pid
            && self.worker_start_time == record.worker_start_time
            && self.child_pid == record.child_pid
            && self.child_start_time == record.child_start_time
            && self.bootstrap_delivery == record.bootstrap_delivery
    }

    fn child_identity(&self) -> ProcessIdentity {
        ProcessIdentity {
            pid: self.child_pid,
            start_time: self.child_start_time,
        }
    }
}
