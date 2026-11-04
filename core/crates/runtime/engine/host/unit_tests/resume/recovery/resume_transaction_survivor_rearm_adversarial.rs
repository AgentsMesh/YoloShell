use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn survivor_rearm_rejects_foreign_tombstone_before_launch_removal() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (manifest, transaction, _) = dead_survivor_rearm_fixture(&host, directory.path());
    let foreign = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        Uuid::new_v4(),
        false,
    );
    host.write_namespace_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &foreign,
    )
    .unwrap();

    assert!(
        host.resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .is_err()
    );
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(manifest.terminal_id)).unwrap(),
        transaction.launch
    );
    assert_eq!(read_resume_tombstone(&host, manifest.terminal_id), foreign);
}

#[tokio::test]
async fn survivor_rearm_rejects_foreign_launch_without_overwriting_it() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (manifest, transaction, _) = dead_survivor_rearm_fixture(&host, directory.path());
    let foreign = shell_resume_launch(&host, &manifest);
    assert_ne!(foreign, transaction.launch);
    host.publish_worker_launch(&foreign).unwrap();

    assert!(
        host.resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .is_err()
    );
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(manifest.terminal_id)).unwrap(),
        foreign
    );
}

#[test]
fn survivor_rearm_evidence_rejects_a_runtime_changed_after_observation() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (mut manifest, transaction, runtime) = dead_survivor_rearm_fixture(&host, directory.path());
    manifest.recovery_state = RecoveryState::Uncertain;
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &manifest)
        .unwrap();
    host.namespace
        .remove_file(&host.paths.worker_launch(manifest.terminal_id))
        .unwrap();
    let mut changed = runtime.clone();
    changed.started_at_ms = changed
        .started_at_ms
        .saturating_add(SURVIVOR_RUNTIME_RACE_OFFSET);
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &changed)
        .unwrap();

    let error = host
        .validate_terminal_rearm_evidence(&manifest, &transaction, &runtime)
        .unwrap_err();
    let detail = error.to_string();
    assert!(detail.contains(crate::host::RESUME_EVIDENCE_MISMATCH), "{detail}");
}

#[test]
fn survivor_rearm_evidence_rejects_a_transaction_changed_after_observation() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (manifest, transaction, runtime) = dead_survivor_rearm_fixture(&host, directory.path());
    host.namespace
        .remove_file(&host.paths.worker_launch(manifest.terminal_id))
        .unwrap();
    let replacement =
        recreate_survivor_transaction(&host, &manifest, transaction.previous_runtime_epoch);
    host.publish_resume_transaction(&replacement).unwrap();

    let error = host
        .validate_terminal_rearm_evidence(&manifest, &transaction, &runtime)
        .unwrap_err();
    let detail = error.to_string();
    assert!(detail.contains(RESUME_TRANSACTION_CONFLICT), "{detail}");
}

#[test]
fn survivor_rearm_evidence_rejects_a_manifest_changed_after_observation() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (manifest, transaction, runtime) = dead_survivor_rearm_fixture(&host, directory.path());
    host.namespace
        .remove_file(&host.paths.worker_launch(manifest.terminal_id))
        .unwrap();
    let mut changed = manifest.clone();
    changed.recovery_state = RecoveryState::Uncertain;
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &changed)
        .unwrap();

    let error = host
        .validate_terminal_rearm_evidence(&manifest, &transaction, &runtime)
        .unwrap_err();
    let detail = error.to_string();
    assert!(detail.contains(RESUME_MANIFEST_MISMATCH), "{detail}");
}
