use super::super::constants::test_field;
use super::super::test_boot::{SOURCE_BOOT, TARGET_BOOT, identity as boot_id};
use super::*;
use crate::model::{BootstrapDelivery, RuntimeRecord};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use std::path::PathBuf;
use uuid::Uuid;

const WORKER_PID: u32 = 61;
const WORKER_START: u64 = 62;
const CHILD_PID: u32 = 63;
const CHILD_START: u64 = 64;
const GENERATION: u64 = 1;
const STARTED_AT_MS: i64 = 1;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const MAX_TEST_FILE_BYTES: u64 = 16_384;
const UNKNOWN_FIELD: &str = "unexpected";
const RUNTIME_FIELD: &str = "runtime";

#[test]
fn production_schema_rejects_unknown_and_wrong_typed_payloads() {
    let (_fixture, intent) = fixture();
    let mut unknown = serde_json::to_value(&intent).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert(UNKNOWN_FIELD.to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<RebootMigrationIntent>(unknown).is_err());
    let mut wrong_type = serde_json::to_value(&intent).unwrap();
    wrong_type[RUNTIME_FIELD][test_field::TARGET_PAYLOAD] =
        serde_json::Value::String("runtime".into());
    assert!(serde_json::from_value::<RebootMigrationIntent>(wrong_type).is_err());
}

#[test]
fn recomputed_target_digest_cannot_authorize_unrelated_runtime_mutation() {
    let (_fixture, intent) = fixture();
    let mut value = serde_json::to_value(&intent).unwrap();
    value[RUNTIME_FIELD][test_field::TARGET_PAYLOAD][test_field::COMMAND] =
        serde_json::Value::String("/bin/foreign-shell".into());
    let payload: RuntimeRecord =
        serde_json::from_value(value[RUNTIME_FIELD][test_field::TARGET_PAYLOAD].clone()).unwrap();
    value[RUNTIME_FIELD][test_field::TARGET_SHA256] =
        serde_json::to_value(crate::sha256_digest::Sha256Digest::capture(&payload).unwrap())
            .unwrap();
    let tampered: RebootMigrationIntent = serde_json::from_value(value).unwrap();
    assert!(validate(&tampered).is_err());
}

struct Fixture {
    _root: tempfile::TempDir,
}

fn fixture() -> (Fixture, RebootMigrationIntent) {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::new(root.path());
    let source_boot = boot_id(SOURCE_BOOT);
    let target_boot = boot_id(TARGET_BOOT);
    let source = runtime(&source_boot);
    paths.ensure_session(source.terminal_id).unwrap();
    let session_path = paths.session_dir(source.terminal_id);
    let namespace = NamespaceLease::capture(&[("reboot validation test", &session_path)]).unwrap();
    namespace
        .write_json(&paths.runtime_record(source.terminal_id), &source)
        .unwrap();
    let checkpoint = checkpoint(&source);
    namespace
        .write_json(
            &paths.worker_startup_checkpoint(source.terminal_id),
            &checkpoint,
        )
        .unwrap();
    let guard =
        super::super::SessionDurableMutationGuard::acquire(&paths, &namespace, source.terminal_id)
            .unwrap();
    let mut target_runtime = source.clone();
    target_runtime.boot_id = target_boot.clone();
    let mut target_checkpoint = checkpoint;
    target_checkpoint.boot_id = target_boot;
    let intent = super::super::prepare_intent(
        &paths,
        &guard,
        super::super::ParticipantTargets {
            startup_checkpoint: target_checkpoint,
            claude_custody: None,
            claude_generation: None,
            pty_custody: None,
            runtime: target_runtime,
        },
    )
    .unwrap();
    drop(guard);
    (Fixture { _root: root }, intent)
}

fn runtime(boot: &str) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START,
        boot_id: boot.to_owned(),
        socket_path: PathBuf::from("/tmp/yolo-reboot.sock"),
        auth_token: "token".to_owned(),
        controller_generation: GENERATION,
        started_at_ms: STARTED_AT_MS,
        command: "/bin/sh".to_owned(),
        args: Vec::new(),
        cwd: PathBuf::from("/tmp"),
        cols: COLUMNS,
        rows: ROWS,
        bootstrap_delivery: BootstrapDelivery::Delivered,
    }
}

fn checkpoint(runtime: &RuntimeRecord) -> crate::worker::StartupCheckpointEvidence {
    let value = crate::worker::StartupCheckpointEvidence {
        schema_version: crate::worker::constants::STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: runtime.controller_generation,
        launch_sha256: crate::sha256_digest::Sha256Digest::repeated(1),
        boot_id: runtime.boot_id.clone(),
        worker: crate::worker::StartupProcessIdentity {
            pid: WORKER_PID,
            start_time: WORKER_START,
        },
        child: Some(crate::worker::StartupProcessIdentity {
            pid: CHILD_PID,
            start_time: CHILD_START,
        }),
        stage: crate::worker::StartupStage::RuntimeActivation,
        phase: crate::worker::StartupPhase::Completed,
        status: crate::worker::StartupStatus::RuntimeActivated,
        successor: None,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    };
    let encoded = serde_json::to_vec(&value).unwrap();
    assert!(encoded.len() as u64 <= MAX_TEST_FILE_BYTES);
    value
}
