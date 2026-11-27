use crate::host::tests::*;
use super::*;

const SOCKET_CLOSE_TEST_TIMEOUT: Duration = PROCESS_EXIT_TIMEOUT;
const SOCKET_CLOSE_TEST_INTERVAL: Duration = PROCESS_EXIT_POLL_INTERVAL;
const IDENTITY_QUERY_FAILURE_DETAIL: &str = "injected process identity query failure";

#[test]
fn identity_query_error_with_alive_fallback_cannot_complete_resume() {
    let indeterminate =
        process_identity_evidence(Err(anyhow::anyhow!(IDENTITY_QUERY_FAILURE_DETAIL)), || true);
    assert_eq!(indeterminate, ProcessIdentityEvidence::Indeterminate);
    assert_ne!(indeterminate, ProcessIdentityEvidence::VerifiedLive);
    let gone =
        process_identity_evidence(Err(anyhow::anyhow!(IDENTITY_QUERY_FAILURE_DETAIL)), || {
            false
        });
    assert_eq!(gone, ProcessIdentityEvidence::ProvenGone);
}

#[tokio::test]
async fn utility_and_persisted_exit_view_paths_are_covered() {
    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let id = Uuid::new_v4();
    paths.ensure_session(id).unwrap();
    let record = RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: u32::MAX,
        worker_start_time: 0,
        child_pid: u32::MAX,
        child_start_time: 1,
        boot_id: "boot".into(),
        socket_path: paths.worker_socket(id).unwrap(),
        auth_token: "token".into(),
        controller_generation: 1,
        started_at_ms: now_ms(),
        command: "/bin/cat".into(),
        args: Vec::new(),
        cwd: directory.path().to_path_buf(),
        cols: 80,
        rows: 24,
        bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
    };
    let exit = crate::model::ExitRecord {
        runtime_epoch: record.runtime_epoch,
        code: Some(7),
        signal: None,
        reason: crate::model::ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: 12,
    };
    atomic_write_json(&paths.exit_record(id), &exit).unwrap();
    let view = unreachable_view(&paths, &record);
    assert_eq!(view.status, TerminalRuntimeStatus::Exited);
    assert_eq!(view.last_output_sequence, 12);
    assert_eq!(command_basename("command"), "command");
    assert!(validate_socket_path(Path::new("/tmp/short.sock")).is_ok());
    assert!(wait_for_process_identity_exit(u32::MAX, 1, Duration::ZERO).await);
    assert!(!process_identity_is_gone(
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap()
    ));
    assert!(process_identity_is_gone(
        std::process::id(),
        process::process_start_time(std::process::id())
            .unwrap()
            .wrapping_add(1)
    ));
    let socket = directory.path().join("probe.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    assert!(socket_may_be_serving(&socket).await);
    drop(listener);
    assert!(wait_for_socket_close(&socket).await);
}

async fn wait_for_socket_close(path: &Path) -> bool {
    tokio::time::timeout(SOCKET_CLOSE_TEST_TIMEOUT, async {
        while socket_may_be_serving(path).await {
            tokio::time::sleep(SOCKET_CLOSE_TEST_INTERVAL).await;
        }
    })
    .await
    .is_ok()
}
