use super::*;
use yolo_process_test_support::{ProcessIdentity as RuntimeProcessIdentity, ResilienceProcessTree};

const RECONCILE_ATTEMPTS: usize = 2;
const LOST_RUNTIME_MISSING_DETAIL: &str = "terminated runtime view is missing";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn durable_termination_without_an_exit_record_never_auto_resurrects() {
    let mut harness = Harness::start().await;
    let terminal_id = harness
        .create_cat("termination-power-loss")
        .await
        .manifest
        .terminal_id;
    let paths = harness.paths();
    let before: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let tombstone =
        yolo_runtime::TerminationTombstone::user_request(terminal_id, before.runtime_epoch, true);
    yolo_runtime::paths::atomic_write_json(&paths.terminate_tombstone(terminal_id), &tombstone)
        .unwrap();
    assert!(!paths.exit_record(terminal_id).exists());

    simulate_power_loss(&mut harness, std::slice::from_ref(&before)).await;
    assert!(!paths.exit_record(terminal_id).exists());
    harness.replace_host(spawn_host(harness.home.path())).await;

    for _ in 0..RECONCILE_ATTEMPTS {
        let result = harness.client().reconcile().await.unwrap();
        assert_eq!(result.reattached, 0);
    }
    let unchanged: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    assert_eq!(unchanged.runtime_epoch, before.runtime_epoch);
    assert_eq!(unchanged.worker_pid, before.worker_pid);
    assert!(!process::process_identity_matches(
        unchanged.worker_pid,
        unchanged.worker_start_time
    ));
    assert!(!paths.exit_record(terminal_id).exists());

    let lost = harness.client().get_session(terminal_id).await.unwrap();
    let lost_runtime = lost.runtime.as_ref().expect(LOST_RUNTIME_MISSING_DETAIL);
    let selection = yolo_runtime::model::TerminalLifecycleSelection::selected(
        terminal_id,
        lost.manifest.updated_at_ms,
        lost_runtime.status,
    )
    .with_expected_manifest_sha256(Some(
        yolo_runtime::model::TerminalManifestDigest::capture(&lost.manifest).unwrap(),
    ))
    .with_expected_runtime_epoch(Some(lost_runtime.runtime_epoch));
    let resumed = harness
        .resume_with_preconditions_and_track(selection, true)
        .await
        .unwrap();
    let replacement: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    assert_ne!(replacement.runtime_epoch, before.runtime_epoch);
    assert_eq!(
        resumed.runtime.unwrap().runtime_epoch,
        replacement.runtime_epoch
    );
    assert!(!paths.terminate_tombstone(terminal_id).exists());
}

async fn kill_runtime_without_finalization(record: &RuntimeRecord) {
    let worker = ProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    };
    let child = ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    let runtime_child = RuntimeProcessIdentity {
        pid: child.pid,
        start_time: child.start_time,
    };
    let mut tree = ResilienceProcessTree::capture(runtime_child).unwrap();
    assert!(worker.signal(libc::SIGKILL).unwrap());
    wait_for_identity_gone(worker).await;
    discard_incomplete_coverage_profile(worker.pid);
    for identity in tree.terminate(libc::SIGKILL).unwrap() {
        wait_for_identity_gone(ProcessIdentity {
            pid: identity.pid,
            start_time: identity.start_time,
        })
        .await;
    }
    wait_for_identity_gone(child).await;
}

include!("termination_recovery/barrier.rs");
include!("termination_recovery/reconcile_first.rs");
