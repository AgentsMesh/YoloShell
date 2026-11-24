use crate::host::tests::*;
use super::*;

const ZOMBIE_POLL_INTERVAL: Duration = Duration::from_millis(5);
const ZOMBIE_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const ZOMBIE_SHELL: &str = "sh";
const ZOMBIE_SHELL_COMMAND: &str = "exit 0";
const ZOMBIE_SHELL_COMMAND_ARGUMENT: &str = "-c";
const ZOMBIE_WAIT_TIMEOUT_DETAIL: &str = "child did not become an unreaped inactive process";
const ZOMBIE_TEST_MISSING_WORKER_PID: u32 = u32::MAX;
const ZOMBIE_TEST_MISSING_WORKER_START_TIME: u64 = 1;

include!("../../test_support/unit/recovery_safety_linux_support.rs");

#[test]
fn unreaped_zombie_without_containment_custody_stays_blocked() {
    let mut child = crate::process::spawn_waitable_test_child(
        std::process::Command::new(ZOMBIE_SHELL)
            .args([ZOMBIE_SHELL_COMMAND_ARGUMENT, ZOMBIE_SHELL_COMMAND]),
    );
    let child_pid = child.id();
    let child_start_time = process::process_start_time(child_pid).unwrap();
    wait_for_inactive_durable_process(child_pid, child_start_time);

    assert!(process_identity_is_gone(child_pid, child_start_time));

    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    runtime.worker_pid = ZOMBIE_TEST_MISSING_WORKER_PID;
    runtime.worker_start_time = ZOMBIE_TEST_MISSING_WORKER_START_TIME;
    runtime.child_pid = child_pid;
    runtime.child_start_time = child_start_time;
    persist_host_session(&host, &manifest, &runtime);
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::BlockedByRuntime
    );
    child.wait().unwrap();
}

#[tokio::test]
async fn dead_worker_live_root_is_settled_before_resume_validation() {
    let Some(broker) = process::linux_containment::LinuxContainmentBroker::discover().unwrap()
    else {
        return;
    };
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut child = spawn_containment_root();
    let root = process_identity(&child);
    let runtime = linux_recovery_runtime(&host, &manifest, root);
    persist_host_session(&host, &manifest, &runtime);
    persist_test_linux_containment(&host, broker, &runtime, root);

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Eligible
    );
    host.validate_previous_resume_runtime_core(&manifest, &runtime)
        .unwrap();
    assert!(
        host.validate_previous_resume_runtime(&manifest, &runtime)
            .is_err()
    );
    host.settle_linux_recreate_runtime(&runtime).await.unwrap();
    host.validate_previous_resume_runtime(&manifest, &runtime)
        .unwrap();
    let record_path = process::linux_containment::record_path(
        &host.paths.session_dir(manifest.terminal_id),
        runtime.runtime_epoch,
    );
    assert!(record_path.exists());
    host.settle_linux_recreate_runtime(&runtime).await.unwrap();
    child.wait().unwrap();
    assert!(!host.paths.terminate_tombstone(manifest.terminal_id).exists());
}

#[tokio::test]
async fn dead_root_descendant_custody_is_settled_idempotently() {
    let Some(broker) = process::linux_containment::LinuxContainmentBroker::discover().unwrap()
    else {
        return;
    };
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let descendant_path = directory.path().join(LINUX_DESCENDANT_PID_FILE);
    let mut root_child = spawn_containment_descendant(&descendant_path);
    let root = process_identity(&root_child);
    let runtime = linux_recovery_runtime(&host, &manifest, root);
    persist_host_session(&host, &manifest, &runtime);
    persist_test_linux_containment(&host, broker, &runtime, root);
    release_containment_descendant(&mut root_child);
    root_child.wait().unwrap();
    let descendant = wait_for_descendant_identity(&descendant_path);

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Eligible
    );
    host.settle_linux_recreate_runtime(&runtime).await.unwrap();
    host.validate_previous_resume_runtime(&manifest, &runtime)
        .unwrap();
    wait_for_process_gone(descendant);
    host.settle_linux_recreate_runtime(&runtime).await.unwrap();
}

fn wait_for_inactive_durable_process(pid: u32, start_time: u64) {
    let deadline = std::time::Instant::now() + ZOMBIE_WAIT_TIMEOUT;
    loop {
        if matches!(process::process_start_time(pid), Ok(actual) if actual == start_time)
            && matches!(
                process::try_process_identity_matches(pid, start_time),
                Ok(false)
            )
        {
            assert!(!process::is_process_alive(pid));
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{}",
            ZOMBIE_WAIT_TIMEOUT_DETAIL
        );
        std::thread::sleep(ZOMBIE_POLL_INTERVAL);
    }
}
