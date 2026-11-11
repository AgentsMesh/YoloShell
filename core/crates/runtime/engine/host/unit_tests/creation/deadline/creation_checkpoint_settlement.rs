use crate::host::tests::*;
use super::*;

include!("../../../test_support/unit/creation/creation_checkpoint_test_support.rs");

#[tokio::test]
async fn creation_checkpoint_absence_allows_prepublication_launch_retry() {
    let fixture = CreationCheckpointFixture::new(false).await;
    assert!(fixture.reconcile().unwrap());
}

#[tokio::test]
async fn creation_checkpoint_presence_requires_a_durable_current_launch() {
    let fixture = CreationCheckpointFixture::new(false).await;
    let checkpoint = fixture.active_checkpoint();
    fixture.write(&checkpoint);

    let error = fixture.reconcile().unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_eq!(fixture.read(), checkpoint);
}

#[tokio::test]
async fn creation_checkpoint_exact_gone_evidence_settles_idempotently() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.active_checkpoint();
    fixture.write(&checkpoint);
    let _gone = gone_checkpoint_processes();

    assert!(fixture.reconcile().unwrap());
    assert_eq!(
        fixture.read().status,
        crate::worker::StartupStatus::HostSettled
    );
    assert!(fixture.reconcile().unwrap());
}

#[tokio::test]
async fn creation_checkpoint_launch_fence_mismatch_is_preserved() {
    let mutations: [fn(&mut crate::worker::StartupCheckpointEvidence); 4] = [
        |value| value.terminal_id = Uuid::new_v4(),
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.controller_generation += CHECKPOINT_TEST_IDENTITY_OFFSET,
    ];
    for mutate in mutations {
        let fixture = CreationCheckpointFixture::new(true).await;
        let mut checkpoint = fixture.active_checkpoint();
        mutate(&mut checkpoint);
        fixture.write(&checkpoint);

        assert!(fixture.reconcile().is_err());
        assert_eq!(fixture.read(), checkpoint);
    }
}

#[tokio::test]
async fn creation_checkpoint_live_or_indeterminate_worker_is_preserved() {
    for relation in [
        crate::process::ProcessIdentityRelation::Matching,
        crate::process::ProcessIdentityRelation::Unknown,
    ] {
        let fixture = CreationCheckpointFixture::new(true).await;
        let checkpoint = fixture.active_checkpoint();
        fixture.write(&checkpoint);
        let _relation = crate::process::install_test_process_identity_relation(
            CHECKPOINT_TEST_PROCESS_PID,
            CHECKPOINT_TEST_START_TIME,
            relation,
        );

        assert!(fixture.reconcile().is_err());
        assert_eq!(fixture.read(), checkpoint);
    }
}

#[tokio::test]
async fn creation_checkpoint_ambiguous_child_entry_is_preserved() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::ChildProcess,
        crate::worker::StartupPhase::Entered,
        crate::worker::StartupStatus::Active,
        None,
    );
    fixture.write(&checkpoint);
    let _gone = gone_checkpoint_processes();

    assert!(fixture.reconcile().is_err());
    assert_eq!(fixture.read(), checkpoint);
}

#[tokio::test]
async fn creation_checkpoint_gone_child_settles_but_live_child_does_not() {
    for child_relation in [
        crate::process::ProcessIdentityRelation::Gone,
        crate::process::ProcessIdentityRelation::Matching,
    ] {
        let fixture = CreationCheckpointFixture::new(true).await;
        let checkpoint = fixture.checkpoint(
            crate::worker::StartupStage::BootstrapGate,
            crate::worker::StartupPhase::Entered,
            crate::worker::StartupStatus::Active,
            Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
        );
        fixture.write(&checkpoint);
        let _worker = crate::process::install_test_process_identity_relation(
            CHECKPOINT_TEST_PROCESS_PID,
            CHECKPOINT_TEST_START_TIME,
            crate::process::ProcessIdentityRelation::Gone,
        );
        let _child = crate::process::install_test_process_identity_relation(
            CHECKPOINT_TEST_CHILD_PID,
            CHECKPOINT_TEST_START_TIME,
            child_relation,
        );

        if child_relation == crate::process::ProcessIdentityRelation::Gone {
            assert!(fixture.reconcile().unwrap());
        } else {
            assert!(fixture.reconcile().is_err());
            assert_eq!(fixture.read(), checkpoint);
        }
    }
}

#[tokio::test]
async fn creation_checkpoint_launch_replacement_preserves_evidence() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.active_checkpoint();
    fixture.write(&checkpoint);
    let mut replacement = fixture.launch.clone();
    replacement.worker_id = Uuid::new_v4();
    let launch_path = fixture
        .host
        .paths
        .worker_launch(fixture.binding.result.terminal_id);
    let _hook = creation_checkpoint_test_hook::install(move || {
        atomic_write_json(&launch_path, &replacement)
    });

    assert!(fixture.reconcile().is_err());
    assert_eq!(fixture.read(), checkpoint);
}

#[tokio::test]
async fn creation_checkpoint_malformed_evidence_is_preserved() {
    let fixture = CreationCheckpointFixture::new(true).await;
    write_private_bytes(&fixture.path(), CHECKPOINT_TEST_MALFORMED_BYTES);

    assert!(fixture.reconcile().is_err());
    assert_eq!(
        std::fs::read(fixture.path()).unwrap(),
        CHECKPOINT_TEST_MALFORMED_BYTES
    );
}

#[tokio::test]
async fn creation_checkpoint_cross_session_settlement_cannot_authorize_retry() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let mut checkpoint = fixture.active_checkpoint();
    checkpoint.terminal_id = Uuid::new_v4();
    checkpoint.status = crate::worker::StartupStatus::HostSettled;
    fixture.write(&checkpoint);

    assert!(fixture.reconcile().is_err());
    assert_eq!(fixture.read(), checkpoint);
}
