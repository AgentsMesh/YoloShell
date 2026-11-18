use super::*;
use crate::process_groups;
use yolo_process_test_support::ProcessIdentityObservation;

const SENTINEL_EXECUTABLE: &str = "/bin/sleep";
const SENTINEL_LIFETIME_SECONDS: &str = "60";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn immediate_successor_binding_reaps_descendants_after_root_and_record_loss() {
    let mut harness = Harness::start().await;
    let (target, _lease, shell, agent, descendant) =
        process_groups::start_stubborn_tree(&harness).await;
    let paths = harness.paths();
    let record: RuntimeRecord = read_json(&paths.runtime_record(target.terminal_id)).unwrap();
    let view = harness
        .client()
        .get_session(target.terminal_id)
        .await
        .unwrap();
    let worker = ProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    };
    let sentinel = ChildGuard::new(
        Command::new(SENTINEL_EXECUTABLE)
            .arg(SENTINEL_LIFETIME_SECONDS)
            .spawn()
            .unwrap(),
    );
    let unrelated = ProcessIdentity::capture(sentinel.id()).unwrap();

    // Durable discovery alone grants no cleanup authority. Authenticate and retain
    // this successor's independent PTY containment before losing the runtime record.
    let (mut cleanup, capture) = RuntimeCleanup::capture(&paths);
    let rejected_discovery = capture.unwrap_err().to_string();
    assert_eq!(
        cleanup.finish().unwrap_err().to_string(),
        rejected_discovery
    );
    cleanup.bind_view(&paths, &view).unwrap();
    assert_eq!(cleanup.bound_records_for_test(), [record.clone()]);

    harness.kill_host_hard();
    assert!(worker.signal(libc::SIGKILL).unwrap());
    wait_for_identity_gone(worker).await;
    discard_incomplete_coverage_profile(worker.pid);
    let _ = shell.signal(libc::SIGKILL).unwrap();
    wait_for_identity_gone(shell).await;
    assert_eq!(agent.observe(), ProcessIdentityObservation::Matching);
    assert_eq!(descendant.observe(), ProcessIdentityObservation::Matching);
    std::fs::remove_file(paths.runtime_record(target.terminal_id)).unwrap();

    cleanup.force();
    cleanup.finish().unwrap();
    for identity in [worker, shell, agent, descendant] {
        assert_eq!(identity.observe(), ProcessIdentityObservation::Gone);
    }
    assert_eq!(unrelated.observe(), ProcessIdentityObservation::Matching);
    assert!(
        !paths
            .runtime_record(target.terminal_id)
            .try_exists()
            .unwrap()
    );
    assert!(!paths.exit_record(target.terminal_id).try_exists().unwrap());
}
