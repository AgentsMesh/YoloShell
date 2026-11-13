use super::super::settlement_retirement::{Progress, publish};
use super::test_support::Fixture;
use crate::worker::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupPhase, StartupProcessIdentity,
    StartupStage, StartupStatus, TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION,
};

const CHECKPOINT_DIGEST_BYTE: u8 = 0x63;
const SUCCESSOR_GENERATION_INCREMENT: u64 = 1;

#[test]
fn host_retired_successor_fence_preserves_predecessor_custody() {
    let fixture = Fixture::new_with_distinct_child(crate::process::ProcessIdentityRelation::Gone);
    let preparation = fixture.install().unwrap();
    let (prepared, _) = fixture.current();
    let (armed, _) =
        super::super::preparation::arm(&fixture.namespace, &fixture.paths, &prepared, preparation)
            .unwrap();
    let before = fixture.current();
    write_host_retired_checkpoint(&fixture);
    let custody = super::test_custody::custody(&fixture);

    let progress = publish(
        &fixture.namespace,
        &fixture.paths,
        &custody,
        &armed,
        &fixture.runtime,
        preparation.binding(),
    )
    .unwrap();

    assert!(matches!(progress, Progress::Pending));
    assert_eq!(fixture.current(), before);
}

fn write_host_retired_checkpoint(fixture: &Fixture) {
    let runtime = &fixture.runtime;
    let checkpoint = StartupCheckpointEvidence {
        schema_version: TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: runtime.controller_generation,
        launch_sha256: crate::sha256_digest::Sha256Digest::repeated(CHECKPOINT_DIGEST_BYTE),
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
        status: StartupStatus::HostRetired,
        successor: Some(StartupLaunchFence {
            terminal_id: runtime.terminal_id,
            runtime_epoch: uuid::Uuid::new_v4(),
            worker_id: uuid::Uuid::new_v4(),
            controller_generation: runtime
                .controller_generation
                .checked_add(SUCCESSOR_GENERATION_INCREMENT)
                .unwrap(),
            launch_sha256: crate::sha256_digest::Sha256Digest::repeated(CHECKPOINT_DIGEST_BYTE),
        }),
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    };
    fixture
        .namespace
        .write_json(
            &fixture.paths.worker_startup_checkpoint(runtime.terminal_id),
            &checkpoint,
        )
        .unwrap();
}
