use super::super::*;
use uuid::Uuid;

const CURRENT_BOOT: &str = "current-boot";
const PREVIOUS_BOOT: &str = "previous-boot";
const UNKNOWN_BOOT: &str = "unknown:boot";
const DEAD_PID: u32 = u32::MAX;
const DEAD_START: u64 = 1;
const FIXTURE_SOCKET: &str = "/tmp/runtime.sock";
const FIXTURE_TOKEN: &str = "runtime-loss-token";
const FIXTURE_COMMAND: &str = "/bin/sh";
const FIXTURE_DIRECTORY: &str = "/tmp";
const FIXTURE_NAME: &str = "runtime-loss";
const FIRST_GENERATION: u64 = 1;
const FIRST_TIMESTAMP: i64 = 1;

fn record(
    boot_id: &str,
    worker_pid: u32,
    worker_start_time: u64,
    child_pid: u32,
    child_start_time: u64,
) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid,
        worker_start_time,
        child_pid,
        child_start_time,
        boot_id: boot_id.into(),
        socket_path: std::path::PathBuf::from(FIXTURE_SOCKET),
        auth_token: FIXTURE_TOKEN.into(),
        controller_generation: FIRST_GENERATION,
        started_at_ms: FIRST_TIMESTAMP,
        command: FIXTURE_COMMAND.into(),
        args: Vec::new(),
        cwd: std::path::PathBuf::from(FIXTURE_DIRECTORY),
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
    }
}

#[test]
fn previous_boot_is_lost_without_probing_reused_process_ids() {
    let current = process::ProcessIdentity::capture(std::process::id()).unwrap();
    let runtime = record(
        PREVIOUS_BOOT,
        current.pid,
        current.start_time,
        current.pid,
        current.start_time,
    );
    assert!(runtime_loss::identities_proven_gone(&runtime, CURRENT_BOOT));
}

#[test]
fn same_boot_requires_both_exact_process_identities_gone() {
    let current = process::ProcessIdentity::capture(std::process::id()).unwrap();
    let live = record(
        CURRENT_BOOT,
        current.pid,
        current.start_time,
        DEAD_PID,
        DEAD_START,
    );
    assert!(!runtime_loss::identities_proven_gone(&live, CURRENT_BOOT));
    let gone = record(CURRENT_BOOT, DEAD_PID, DEAD_START, DEAD_PID, DEAD_START);
    assert!(runtime_loss::identities_proven_gone(&gone, CURRENT_BOOT));
}

#[test]
fn unknown_boot_never_becomes_lost() {
    let gone = record(UNKNOWN_BOOT, DEAD_PID, DEAD_START, DEAD_PID, DEAD_START);
    assert!(!runtime_loss::identities_proven_gone(&gone, CURRENT_BOOT));
}

#[tokio::test]
async fn lost_runtime_reads_preserve_recovery_policy_and_automatic_restore_eligibility() {
    let temporary = tempfile::tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(temporary.path())).unwrap();
    host.boot_id = CURRENT_BOOT.into();
    let current = process::ProcessIdentity::capture(std::process::id()).unwrap();
    let mut runtime = record(
        PREVIOUS_BOOT,
        current.pid,
        current.start_time,
        current.pid,
        current.start_time,
    );
    runtime.cwd = temporary.path().to_path_buf();
    runtime.socket_path = host.paths.worker_socket(runtime.terminal_id).unwrap();
    let manifest = TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        name: FIXTURE_NAME.into(),
        command: runtime.command.clone(),
        args: runtime.args.clone(),
        cwd: runtime.cwd.clone(),
        cols: runtime.cols,
        rows: runtime.rows,
        created_at_ms: FIRST_TIMESTAMP,
        updated_at_ms: FIRST_TIMESTAMP,
        archived: false,
        recovery_state: RecoveryState::Reattached,
    };
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let manifest_path = host.paths.manifest(manifest.terminal_id);
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    host.write_namespace_json(&manifest_path, &manifest)
        .unwrap();
    host.write_namespace_json(&runtime_path, &runtime).unwrap();
    host.catalog_insert(&manifest).await.unwrap();
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Eligible
    );
    let before_manifest = std::fs::read(&manifest_path).unwrap();
    let before_runtime = std::fs::read(&runtime_path).unwrap();
    let observed = host.view_for_manifest(manifest.clone()).await;
    assert_eq!(observed.runtime.unwrap().status, TerminalRuntimeStatus::Lost);
    assert_eq!(observed.manifest.recovery_state, manifest.recovery_state);
    let observed = host.get_session_view_for_manifest(manifest.clone()).await;
    assert_eq!(observed.runtime.unwrap().status, TerminalRuntimeStatus::Lost);
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Eligible
    );
    assert_eq!(std::fs::read(manifest_path).unwrap(), before_manifest);
    assert_eq!(std::fs::read(runtime_path).unwrap(), before_runtime);
    assert_eq!(
        host.catalog_get(manifest.terminal_id)
            .await
            .unwrap()
            .unwrap()
            .recovery_state,
        manifest.recovery_state
    );
    assert!(!host.paths.exit_record(manifest.terminal_id).exists());
    assert!(current.matches());
}
