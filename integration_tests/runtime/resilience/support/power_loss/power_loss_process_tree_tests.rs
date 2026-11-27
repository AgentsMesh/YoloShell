use super::*;
use crate::process_groups;

#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
#[path = "power_loss_pre_capture_escape_tests.rs"]
mod pre_capture_escape_tests;

const PROCESS_GROUP_UNAVAILABLE_DETAIL: &str = "process group unavailable";
const LOST_RUNTIME_MISSING_DETAIL: &str = "lost runtime view is missing";
const RESUMED_RUNTIME_MISSING_DETAIL: &str = "explicit recovery did not return a runtime";
const WORKER_KILL_CONTEXT: &str = "kill worker before root-disappearance cleanup";
const SHELL_KILL_CONTEXT: &str = "kill shell before root-disappearance cleanup";
const PREVIOUS_POWER_LOSS_BOOT_ID: &str = "previous-power-loss-boot";
const EXIT_EVIDENCE_REASON: yolo_runtime::model::ExitReason =
    yolo_runtime::model::ExitReason::HostReboot;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn power_loss_reaps_foreground_agent_and_descendant_after_worker_loss() {
    let mut harness = Harness::start().await;
    let (target, _lease, shell, agent, descendant) =
        process_groups::start_stubborn_tree(&harness).await;
    let paths = harness.paths();
    let record: RuntimeRecord = read_json(&paths.runtime_record(target.terminal_id)).unwrap();

    assert_ne!(process_group(agent.pid), process_group(shell.pid));
    simulate_power_loss(&mut harness, std::slice::from_ref(&record)).await;

    assert_runtime_identities_gone(&record);
    assert!(!agent.matches());
    assert!(!descendant.matches());
    assert!(!paths.exit_record(target.terminal_id).try_exists().unwrap());
    yolo_runtime::durable_reboot::adversarial::persist_previous_boot(
        &paths,
        &record,
        PREVIOUS_POWER_LOSS_BOOT_ID,
    )
    .unwrap();
    harness.replace_host(spawn_host(harness.home.path())).await;
    let client = harness.client();
    client.reconcile().await.unwrap();
    let lost = client.get_session(target.terminal_id).await.unwrap();
    let lost_runtime = lost.runtime.as_ref().expect(LOST_RUNTIME_MISSING_DETAIL);
    let selection = yolo_runtime::model::TerminalLifecycleSelection::selected(
        target.terminal_id,
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
    let runtime = resumed
        .runtime
        .as_ref()
        .expect(RESUMED_RUNTIME_MISSING_DETAIL);
    let recovered = recovered_runtime(&record, runtime);
    drop(client);
    drop(harness);
    wait_for_identity_gone(recovered.0).await;
    wait_for_identity_gone(recovered.1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn harness_cleanup_reaps_reparented_descendants_after_the_root_disappears() {
    let mut harness = Harness::start().await;
    let (target, _lease, shell, agent, descendant) =
        process_groups::start_stubborn_tree(&harness).await;
    let record: RuntimeRecord =
        read_json(&harness.paths().runtime_record(target.terminal_id)).unwrap();
    let worker = runtime_worker(&record);

    let paused = guardian_pause::PausedGuardian::capture(&harness, &record).await;
    harness.kill_host_hard();
    assert!(worker.signal(libc::SIGKILL).expect(WORKER_KILL_CONTEXT));
    wait_for_identity_gone(worker).await;
    discard_incomplete_coverage_profile(worker.pid);
    let _ = shell.signal(libc::SIGKILL).expect(SHELL_KILL_CONTEXT);
    wait_for_identity_gone(shell).await;
    assert!(agent.matches());
    assert!(descendant.matches());

    drop(paused);
    drop(harness);
    wait_for_identity_gone(agent).await;
    wait_for_identity_gone(descendant).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exit_evidence_failure_preserves_the_primary_failure_and_cleanup_custody() {
    let mut harness = Harness::start().await;
    let (target, _lease, shell, agent, descendant) =
        process_groups::start_stubborn_tree(&harness).await;
    let paths = harness.paths();
    let record: RuntimeRecord = read_json(&paths.runtime_record(target.terminal_id)).unwrap();
    let worker = runtime_worker(&record);
    harness.kill_host_hard();
    assert!(worker.signal(libc::SIGKILL).expect(WORKER_KILL_CONTEXT));
    wait_for_identity_gone(worker).await;
    discard_incomplete_coverage_profile(worker.pid);
    let exit_path = paths.exit_record(target.terminal_id);
    write_exit_evidence(&exit_path, record.runtime_epoch);
    let evidence = std::fs::read(&exit_path).unwrap();
    let bound = harness.bind_cleanup_records(std::slice::from_ref(&record));
    let failure = tokio::spawn(async move {
        simulate_bound_power_loss(
            &paths,
            &bound,
            GuardianExpectation::RequiredOnSupportedPlatform,
        )
        .await;
    })
    .await
    .unwrap_err();

    assert!(failure.is_panic());
    assert_eq!(std::fs::read(&exit_path).unwrap(), evidence);
    let retained = harness.bind_cleanup_records(std::slice::from_ref(&record));
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].record(), &record);
    retained[0].verify_runtime_file().unwrap();
    assert!(
        harness
            .cleanup_trees
            .lock()
            .unwrap()
            .has_root(runtime_child(&retained[0]))
    );
    drop(harness);
    wait_for_identity_gone(shell).await;
    wait_for_identity_gone(agent).await;
    wait_for_identity_gone(descendant).await;
}

fn recovered_runtime(
    previous: &RuntimeRecord,
    runtime: &yolo_runtime::model::RuntimeView,
) -> (ProcessIdentity, ProcessIdentity) {
    assert_eq!(runtime.status, TerminalRuntimeStatus::Running);
    assert_runtime_identity_rotated(previous, runtime);
    (
        ProcessIdentity {
            pid: runtime.worker_pid,
            start_time: runtime.worker_start_time,
        },
        ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        },
    )
}

fn process_group(pid: u32) -> i32 {
    let group = unsafe { libc::getpgid(pid as i32) };
    assert!(group > 0, "{PROCESS_GROUP_UNAVAILABLE_DETAIL}: {pid}");
    group
}

fn runtime_worker(record: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    }
}

fn write_exit_evidence(path: &Path, runtime_epoch: Uuid) {
    let evidence = yolo_runtime::model::ExitRecord {
        runtime_epoch,
        code: None,
        signal: None,
        reason: EXIT_EVIDENCE_REASON,
        observed_at_ms: i64::MIN,
        last_output_sequence: u64::MIN,
    };
    yolo_runtime::paths::atomic_write_json(path, &evidence).unwrap();
}
