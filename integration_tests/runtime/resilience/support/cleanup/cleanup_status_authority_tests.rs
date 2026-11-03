use super::runtime_record_authority::BoundRuntimeRecord;
use super::runtime_record_test_support::{record_for_identity, runtime_view, session_view};
use super::*;

const TEST_PROCESS_ARGUMENT: &str = "60";
const TEST_PROCESS_EXECUTABLE: &str = "/bin/sleep";

#[test]
fn live_worker_states_can_establish_cleanup_authority() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    for status in [
        TerminalRuntimeStatus::Starting,
        TerminalRuntimeStatus::Running,
        TerminalRuntimeStatus::Finalizing,
        TerminalRuntimeStatus::OutputBlocked,
    ] {
        let mut view = runtime_view(&record);
        view.status = status;
        BoundRuntimeRecord::bind_runtime(&paths, record.clone(), record.terminal_id, &view)
            .unwrap();
    }
}

#[test]
fn non_live_states_never_authorize_cleanup_signals() {
    for status in [
        TerminalRuntimeStatus::Unreachable,
        TerminalRuntimeStatus::Exited,
    ] {
        assert_synthetic_state_cannot_signal(status);
    }
}

#[test]
fn previously_bound_runtime_can_refresh_to_an_exited_view() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let running = runtime_view(&record);
    let bound =
        BoundRuntimeRecord::bind_runtime(&paths, record.clone(), record.terminal_id, &running)
            .unwrap();
    let mut exited = session_view(&record);
    exited.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Exited;

    bound.refresh_view(&record, &exited).unwrap();
}

fn assert_synthetic_state_cannot_signal(status: TerminalRuntimeStatus) {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let (mut cleanup, capture) = RuntimeCleanup::capture(&paths);
    capture.unwrap();
    let mut victim = ChildGuard::new(
        Command::new(TEST_PROCESS_EXECUTABLE)
            .arg(TEST_PROCESS_ARGUMENT)
            .spawn()
            .unwrap(),
    );
    let identity = ProcessIdentity::capture(victim.id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let mut view = session_view(&record);
    view.runtime.as_mut().unwrap().status = status;

    assert!(cleanup.bind_view(&paths, &view).is_err());
    cleanup.request();
    cleanup.force();
    assert!(cleanup.finish().is_err());
    assert!(identity.matches());

    victim.kill().unwrap();
    victim.wait().unwrap();
}
