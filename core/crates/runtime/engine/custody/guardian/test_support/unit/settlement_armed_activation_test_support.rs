use super::Fixture;
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::sha256_digest::Sha256Digest;
use crate::worker::{
    StartupCheckpointEvidence, StartupPhase, StartupProcessIdentity, StartupStage, StartupStatus,
    TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION,
};

const CHECKPOINT_DIGEST_BYTE: u8 = 0x52;
const GENERATION_INCREMENT: u64 = 1;

pub(super) fn successor_generation(runtime: &RuntimeRecord) -> u64 {
    runtime
        .controller_generation
        .checked_add(GENERATION_INCREMENT)
        .unwrap()
}

pub(in crate::custody_guardian::settlement_armed) fn publish_runtime(
    fixture: &Fixture,
    runtime: &RuntimeRecord,
) -> FileIdentity {
    let path = fixture.paths.runtime_record(runtime.terminal_id);
    fixture.namespace.write_json(&path, runtime).unwrap();
    let observed = fixture
        .namespace
        .read_json_with_identity::<RuntimeRecord>(&path)
        .unwrap();
    assert_eq!(observed.0, *runtime);
    observed.1
}

pub(super) fn rewrite_runtime_in_place(fixture: &Fixture, runtime: &RuntimeRecord) -> FileIdentity {
    let path = fixture.paths.runtime_record(runtime.terminal_id);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    serde_json::to_writer(&mut file, runtime).unwrap();
    file.sync_all().unwrap();
    drop(file);
    let observed = fixture
        .namespace
        .read_json_with_identity::<RuntimeRecord>(&path)
        .unwrap();
    assert_eq!(observed.0, *runtime);
    assert!(observed.1.same_object(fixture.runtime_identity));
    observed.1
}

pub(super) fn write_activated_checkpoint(fixture: &Fixture, runtime: &RuntimeRecord) {
    let checkpoint = StartupCheckpointEvidence {
        schema_version: TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: runtime.controller_generation,
        launch_sha256: Sha256Digest::repeated(CHECKPOINT_DIGEST_BYTE),
        boot_id: runtime.boot_id.clone(),
        worker: StartupProcessIdentity {
            pid: runtime.worker_pid,
            start_time: runtime.worker_start_time,
        },
        child: Some(StartupProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        }),
        stage: StartupStage::RuntimeActivation,
        phase: StartupPhase::Completed,
        status: StartupStatus::RuntimeActivated,
        successor: None,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    };
    let path = fixture.paths.worker_startup_checkpoint(runtime.terminal_id);
    fixture.namespace.write_json(&path, &checkpoint).unwrap();
}
