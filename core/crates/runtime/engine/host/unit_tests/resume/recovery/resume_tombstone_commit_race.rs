use crate::host::tests::*;
use super::*;

#[cfg(target_os = "macos")]
#[tokio::test]
async fn termination_during_crash_exit_commit_blocks_publish_without_uncertainty() {
    let fixture = AutomaticCrashFixture::new();
    fixture.append_output();
    fixture.settle_custody();
    let tombstone = crate::model::TerminationTombstone::user_request(
        fixture.runtime.terminal_id,
        fixture.runtime.runtime_epoch,
        true,
    );
    let expected_tombstone = tombstone.clone();
    let tombstone_path = fixture
        .host
        .paths
        .terminate_tombstone(fixture.runtime.terminal_id);
    let _guard =
        automatic_worker_crash_exit_test_hook::install(fixture.runtime.terminal_id, move || {
            atomic_write_json(&tombstone_path, &tombstone)
        });

    let result = fixture
        .host
        .reconcile_manifest(fixture.manifest.clone())
        .await;

    assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(result.needs_recreate, usize::default());
    assert!(
        !fixture
            .host
            .paths
            .resume_transaction(fixture.runtime.terminal_id)
            .exists()
    );
    assert!(
        !fixture
            .host
            .paths
            .worker_launch(fixture.runtime.terminal_id)
            .exists()
    );
    let stored = fixture
        .host
        .catalog_get(fixture.runtime.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.recovery_state, fixture.manifest.recovery_state);
    let exit: ExitRecord =
        read_json(&fixture.host.paths.exit_record(fixture.runtime.terminal_id)).unwrap();
    assert_eq!(exit.reason, ExitReason::WorkerCrash);
    assert_eq!(
        read_json::<crate::model::TerminationTombstone>(
            &fixture
                .host
                .paths
                .terminate_tombstone(fixture.runtime.terminal_id),
        )
        .unwrap(),
        expected_tombstone
    );
}

#[cfg(target_os = "macos")]
#[test]
fn user_termination_block_requires_the_exact_expected_runtime() {
    let fixture = AutomaticCrashFixture::new();
    let tombstone = crate::model::TerminationTombstone::user_request(
        fixture.runtime.terminal_id,
        fixture.runtime.runtime_epoch,
        true,
    );
    fixture
        .host
        .write_namespace_json(
            &fixture
                .host
                .paths
                .terminate_tombstone(fixture.runtime.terminal_id),
            &tombstone,
        )
        .unwrap();
    assert_eq!(
        fixture
            .host
            .automatic_resume_block(&fixture.manifest, &fixture.runtime),
        AutomaticResumeBlock::UserTermination
    );

    let mut replacement = fixture.runtime.clone();
    replacement.worker_id = Uuid::new_v4();
    fixture
        .host
        .write_namespace_json(
            &fixture
                .host
                .paths
                .runtime_record(fixture.runtime.terminal_id),
            &replacement,
        )
        .unwrap();
    assert_eq!(
        fixture
            .host
            .automatic_resume_block(&fixture.manifest, &fixture.runtime),
        AutomaticResumeBlock::RecoveryEvidenceChanged
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn post_crash_exit_revalidation_classifies_exact_custody_evidence() {
    let mutations = [
        PostCrashExitMutation::MatchingTombstone,
        PostCrashExitMutation::ForeignEpochTombstone,
        PostCrashExitMutation::CorruptTombstone,
        PostCrashExitMutation::RuntimeReplacement,
    ];
    for mutation in mutations {
        let fixture = AutomaticCrashFixture::new();
        fixture.append_output();
        fixture.settle_custody();
        let mutation_runtime = fixture.runtime.clone();
        let tombstone_path = fixture
            .host
            .paths
            .terminate_tombstone(fixture.runtime.terminal_id);
        let runtime_path = fixture
            .host
            .paths
            .runtime_record(fixture.runtime.terminal_id);
        *lock_recover(&fixture.host.automatic_resume_test_probes.after_crash_exit) =
            Some(Box::new(move || {
                mutation.apply(&tombstone_path, &runtime_path, &mutation_runtime);
            }));

        let result = fixture
            .host
            .begin_resume_transaction(
                fixture.manifest.clone(),
                ResumeSelection::Preserve(ResumeIntent::Automatic),
                None,
            )
            .await;
        assert!(matches!(
            result,
            Err(SessionResumeFailure::AutomaticBlocked(block))
                if block == mutation.expected_block()
        ));
    }
}
