use super::*;
use crate::host::tests::*;

const LEGACY_TERMINAL_COLUMNS: u16 = 1;
const LEGACY_TERMINAL_ROWS: u16 = 1;
const LEGACY_TEST_WORKER_PID: u32 = 31;
pub(super) const LEGACY_TEST_CHILD_PID: u32 = 32;
pub(super) const LEGACY_TEST_PROCESS_START: u64 = 41;

#[tokio::test]
async fn failed_create_persists_explicit_name_and_clamped_dimensions() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();
    let error = host
        .create_session(CreateTerminal {
            creation_token: Uuid::new_v4(),
            name: Some("named-shell".into()),
            cwd: directory.path().to_path_buf(),
            cols: Some(0),
            rows: Some(0),
            initial_input: None,
            context: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    let manifest = host.catalog.lock().unwrap().list(false).unwrap().remove(0);
    assert_eq!(manifest.name, "named-shell");
    assert_eq!(
        (manifest.cols, manifest.rows),
        (
            crate::model::MIN_TERMINAL_COLUMNS,
            crate::model::MIN_TERMINAL_ROWS,
        )
    );
}

#[test]
fn historical_single_column_evidence_normalizes_across_launch_and_runtime() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.cols = LEGACY_TERMINAL_COLUMNS;
    manifest.rows = LEGACY_TERMINAL_ROWS;
    let safe_launch = shell_resume_launch(&host, &manifest);
    assert_eq!(safe_launch.cols, crate::model::MIN_TERMINAL_COLUMNS);

    let mut historical_launch = safe_launch;
    historical_launch.cols = LEGACY_TERMINAL_COLUMNS;
    historical_launch.rows = LEGACY_TERMINAL_ROWS;
    let transaction = shell_resume_transaction(Uuid::new_v4(), &historical_launch);
    let mut runtime = planned_resume_runtime(
        &host,
        &transaction,
        LEGACY_TEST_WORKER_PID,
        LEGACY_TEST_PROCESS_START,
        LEGACY_TEST_CHILD_PID,
        LEGACY_TEST_PROCESS_START,
        BootstrapDelivery::NotRequired,
    );
    runtime.cols = crate::model::MIN_TERMINAL_COLUMNS;
    runtime.rows = crate::model::MIN_TERMINAL_ROWS;

    host.validate_planned_runtime_core(&manifest, &transaction, &runtime)
        .unwrap();
}

#[tokio::test]
async fn resume_without_a_runtime_record_is_fenced_as_uncertain() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();

    for recreate in [false, true] {
        let manifest = fixture_manifest(directory.path());
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        host.catalog.lock().unwrap().insert(&manifest).unwrap();
        atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
        AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id))
            .save(&host_binding(manifest.terminal_id))
            .unwrap();
        let error = host
            .resume_session(ResumeRequest {
                selection: TerminalLifecycleSelection::unconditional(manifest.terminal_id),
                recreate,
            })
            .await
            .unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
        let stored = host
            .catalog
            .lock()
            .unwrap()
            .get(manifest.terminal_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            stored.recovery_state,
            if recreate {
                RecoveryState::Created
            } else {
                RecoveryState::Uncertain
            }
        );
    }
}

#[tokio::test]
async fn create_session_rejects_nil_token_before_any_durable_side_effect() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let error = host
        .create_session(CreateTerminal {
            creation_token: Uuid::nil(),
            name: Some("nil-token".into()),
            cwd: directory.path().to_path_buf(),
            cols: None,
            rows: None,
            initial_input: None,
            context: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INVALID_REQUEST);
    assert!(
        lock_recover(&host.creation_capabilities)
            .bindings
            .is_empty()
    );
    assert!(lock_recover(&host.catalog).list(false).unwrap().is_empty());
}
