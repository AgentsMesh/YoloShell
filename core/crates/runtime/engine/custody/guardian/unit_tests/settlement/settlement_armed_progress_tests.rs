use super::super::settlement::{GuardianState, Progress, progress_record};
use super::activation_tests::support::publish_runtime;
use super::test_support::Fixture;
use crate::model::BootstrapDelivery;
use crate::process::ProcessIdentityRelation;
use crate::worker::{
    StartupCheckpointEvidence, StartupPhase, StartupProcessIdentity, StartupStage, StartupStatus,
    TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION,
};

const CHECKPOINT_DIGEST_BYTE: u8 = 0x41;
const PROMOTED_RUNTIME_OBJECT_DETAIL: &str =
    "custody promotion reused the retained runtime file object";
const ACTIVATED_RUNTIME_OBJECT_DETAIL: &str =
    "bootstrap activation reused a retained runtime file object";

#[test]
fn armed_probe_precedes_activation_and_completed_promotion_is_verified_once() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let preparation = fixture.install().unwrap();
    let (prepared, _) = fixture.current();
    let mut state = GuardianState::new(
        preparation,
        prepared.clone(),
        fixture.snapshot.clone(),
        fixture.runtime.clone(),
        fixture.runtime_identity,
        fixture.retained_runtime_file(),
    )
    .unwrap();
    let (armed, armed_identity) = crate::custody_guardian::preparation::arm(
        &fixture.namespace,
        &fixture.paths,
        &prepared,
        preparation,
    )
    .unwrap();
    write_checkpoint(&fixture, StartupStatus::Active);
    let custody = super::test_custody::custody(&fixture);
    let expected_probe = custody.master_fd_custody_probe().unwrap();

    let lifecycle = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.runtime.terminal_id)
        .unwrap()
        .unwrap();
    let progress = progress_record(
        &fixture.namespace,
        &fixture.paths,
        &custody,
        &mut state,
        armed.clone(),
        armed_identity,
    )
    .unwrap_or_else(|error| panic!("{error:#}"));
    assert!(matches!(progress, Progress::Pending));
    assert!(state.needs_armed_promotion());
    assert_eq!(
        super::super::test_probe::read_exact(
            &fixture.namespace,
            &fixture.paths,
            &armed,
            armed_identity,
        )
        .unwrap(),
        expected_probe,
    );
    drop(lifecycle);

    let mut activated = crate::custody_promotion::publish_exact(
        &fixture.namespace,
        &fixture.paths,
        &armed,
        armed_identity,
    )
    .unwrap();
    let promoted_identity = fixture
        .namespace
        .read_json_with_identity::<crate::model::RuntimeRecord>(
            &fixture.paths.runtime_record(fixture.runtime.terminal_id),
        )
        .unwrap()
        .1;
    assert!(
        !promoted_identity.same_object(fixture.runtime_identity),
        "{PROMOTED_RUNTIME_OBJECT_DETAIL}: retained={:?}, promoted={promoted_identity:?}",
        fixture.runtime_identity,
    );
    activated.bootstrap_delivery = BootstrapDelivery::Delivered;
    let activated_identity = publish_runtime(&fixture, &activated);
    assert!(
        !activated_identity.same_object(fixture.runtime_identity)
            && !activated_identity.same_object(promoted_identity),
        "{ACTIVATED_RUNTIME_OBJECT_DETAIL}: retained={:?}, promoted={promoted_identity:?}, activated={activated_identity:?}",
        fixture.runtime_identity,
    );
    write_checkpoint_for_runtime(&fixture, &activated, StartupStatus::RuntimeActivated);
    let progress = progress_record(
        &fixture.namespace,
        &fixture.paths,
        &custody,
        &mut state,
        armed.clone(),
        armed_identity,
    )
    .unwrap_or_else(|error| panic!("{error:#}"));
    assert!(matches!(progress, Progress::Pending));
    assert!(!state.needs_armed_promotion());
    assert_eq!(
        state.retained_runtime().unwrap(),
        (&activated, activated_identity)
    );

    let _lifecycle = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.runtime.terminal_id)
        .unwrap()
        .unwrap();
    assert!(matches!(
        progress_record(
            &fixture.namespace,
            &fixture.paths,
            &custody,
            &mut state,
            armed,
            armed_identity,
        ),
        Ok(Progress::Pending)
    ));
}

fn write_checkpoint(fixture: &Fixture, status: StartupStatus) {
    write_checkpoint_for_runtime(fixture, &fixture.runtime, status);
}

fn write_checkpoint_for_runtime(
    fixture: &Fixture,
    runtime: &crate::model::RuntimeRecord,
    status: StartupStatus,
) {
    let activated = status == StartupStatus::RuntimeActivated;
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
        phase: if activated {
            StartupPhase::Completed
        } else {
            StartupPhase::Entered
        },
        status,
        successor: None,
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
