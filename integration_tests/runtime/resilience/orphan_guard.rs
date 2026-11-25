use super::*;
use yolo_process_test_support::ProcessIdentityObservation;

#[path = "orphan_guard_lifecycle.rs"]
mod lifecycle;
use lifecycle::*;
#[path = "orphan_guard_harness.rs"]
mod harness_retirement;
#[path = "test_support/orphan_socket_root.rs"]
mod isolated_namespace;

const TERM_IGNORED_MARKER: &[u8] = b"term-ignored-ready";
const STOPPED_TERM_IGNORING_COMMAND: &[u8] = b"trap '' TERM; printf \
    'term-ignored-ready\\n'; kill -STOP $$; while :; do sleep 1; done\n";

#[tokio::test(flavor = "multi_thread")]
async fn worker_and_child_exit_when_the_session_persistence_root_is_removed() {
    if !isolated_namespace::is_child() {
        isolated_namespace::run().await.unwrap();
        return;
    }
    let mut harness = Harness::start().await;
    let session = harness.create_cat("orphan-guard").await;
    let terminal_id = session.manifest.terminal_id;
    let runtime = session.runtime.expect("running session runtime");
    prepare_term_ignoring_child(&harness, runtime_ref(terminal_id, runtime.runtime_epoch)).await;
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    let record: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    let worker = ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    };
    let child = ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    };
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    let (guardian, guardian_custody) =
        active_guardian(&harness, terminal_id, record.controller_generation).await;

    let displaced_home = harness.retire_namespace_for_test();
    let mut cleanup = OrphanCleanup::processes([worker, child]);
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    cleanup.track(guardian);

    let exited = tokio::time::timeout(
        GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT,
        wait_for_orphan_settlement(
            worker,
            child,
            #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
            guardian,
            #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
            guardian_custody,
        ),
    )
    .await;
    let worker_observation = worker.observe();
    let child_observation = child.observe();
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    let guardian_observation = guardian.observe();
    let host_stderr = harness.host_stderr_tail();
    assert!(
        exited.is_ok(),
        "orphaned runtime custody must settle: worker={:?}, child={:?}, guardian={:?}, host_stderr={}",
        worker_observation,
        child_observation,
        guardian_diagnostic(
            #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
            guardian_observation,
        ),
        host_stderr
    );
    drop(cleanup);
    drop(harness);
    assert!(!displaced_home.exists());
}

async fn prepare_term_ignoring_child(harness: &Harness, target: RuntimeRef) {
    let terminal_id = target.terminal_id;
    let attachment = harness.client().attach(terminal_id, 0).await.unwrap();
    let lease = harness
        .client()
        .acquire_writer(target, "cleanup-writer", None)
        .await
        .unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            STOPPED_TERM_IGNORING_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    read_until(attachment, TERM_IGNORED_MARKER).await;
}
