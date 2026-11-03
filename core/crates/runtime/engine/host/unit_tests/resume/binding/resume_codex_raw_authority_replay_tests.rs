use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn raw_authority_remove_commit_replays_after_power_loss() {
    let (_directory, host, manifest, store, transaction, marker) =
        raw_authority_replay_fixture().await;
    let (_, _, evidence) = binding_to_clear(&transaction).unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &host.paths,
        &host.namespace,
        manifest.terminal_id,
    )
    .unwrap();

    host.revoke_resume_codex_authority(&guard, &transaction, &marker, evidence.as_ref())
        .unwrap();
    drop(guard);

    let authority_path = store.codex_authority_store().unwrap().path().to_path_buf();
    assert!(host.namespace.file_is_missing(&authority_path).unwrap());
    assert!(store.path().exists());
    assert_eq!(
        host.read_namespace_json::<ResumeBindingClearMarker>(
            &host.paths.resume_binding_clear(manifest.terminal_id),
        )
        .unwrap()
        .phase,
        ResumeBindingClearPhase::Pending
    );

    host.stage_resume_binding_disposition(&transaction).unwrap();

    assert!(host.namespace.file_is_missing(&authority_path).unwrap());
    assert!(host.namespace.file_is_missing(store.path()).unwrap());
    assert_eq!(
        host.read_namespace_json::<ResumeBindingClearMarker>(
            &host.paths.resume_binding_clear(manifest.terminal_id),
        )
        .unwrap()
        .phase,
        ResumeBindingClearPhase::Cleared
    );
}

#[tokio::test]
async fn raw_authority_remove_replay_republishes_launch_before_staging() {
    let (_directory, host, manifest, store, transaction, marker) =
        raw_authority_replay_fixture().await;
    let (_, _, evidence) = binding_to_clear(&transaction).unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &host.paths,
        &host.namespace,
        manifest.terminal_id,
    )
    .unwrap();
    host.revoke_resume_codex_authority(&guard, &transaction, &marker, evidence.as_ref())
        .unwrap();
    drop(guard);
    host.namespace
        .remove_file(&host.paths.worker_launch(manifest.terminal_id))
        .unwrap();

    let launch = host
        .stage_resume_binding_disposition_with_replay_launch(&transaction)
        .unwrap();

    assert_eq!(launch, transaction.launch);
    assert_eq!(
        host.read_namespace_json::<WorkerLaunchSpec>(
            &host.paths.worker_launch(manifest.terminal_id)
        )
        .unwrap(),
        transaction.launch
    );
    assert!(
        host.namespace
            .file_is_missing(store.codex_authority_store().unwrap().path())
            .unwrap()
    );
    assert!(host.namespace.file_is_missing(store.path()).unwrap());
    assert_eq!(
        host.read_namespace_json::<ResumeBindingClearMarker>(
            &host.paths.resume_binding_clear(manifest.terminal_id),
        )
        .unwrap()
        .phase,
        ResumeBindingClearPhase::Cleared
    );
}

#[tokio::test]
async fn raw_authority_absence_replacement_aba_fails_closed() {
    let (_directory, host, manifest, store, transaction, marker) =
        raw_authority_replay_fixture().await;
    let authority_store = store.codex_authority_store().unwrap();
    let captured = raw_authority_evidence(&transaction);
    host.namespace.remove_file(authority_store.path()).unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &host.paths,
        &host.namespace,
        manifest.terminal_id,
    )
    .unwrap();
    let injected = std::cell::Cell::new(false);
    let malformed = serde_json::json!({});

    let result = host.remove_raw_codex_authority_with_boundary(
        &guard,
        &authority_store,
        captured,
        &transaction,
        &marker,
        || {
            if !injected.replace(true) {
                atomic_write_json(authority_store.path(), &malformed)?;
            }
            Ok(())
        },
    );

    assert!(result.is_err());
    let (_, replacement) = host
        .namespace
        .read_json_with_identity::<serde_json::Value>(authority_store.path())
        .unwrap();
    assert!(!replacement.unchanged(captured));
    assert_eq!(
        read_json::<serde_json::Value>(authority_store.path()).unwrap(),
        malformed
    );
    assert!(store.path().exists());
}

pub(super) async fn raw_authority_replay_fixture() -> (
    tempfile::TempDir,
    Host,
    TerminalManifest,
    AgentBindingStore,
    ResumeTransaction,
    ResumeBindingClearMarker,
) {
    let (directory, host, mut manifest, binding, store) = codex_repair_fixture();
    ProviderAuthorityPublisher::new()
        .publish(&host, &binding)
        .unwrap();
    let authority_store = store.codex_authority_store().unwrap();
    atomic_write_json(authority_store.path(), &serde_json::json!({})).unwrap();
    let transaction = codex_recreate_transaction(&host, &mut manifest).await;
    host.publish_resume_transaction(&transaction).unwrap();
    host.publish_worker_launch(&transaction.launch).unwrap();
    let marker = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    host.write_namespace_json(
        &host.paths.resume_binding_clear(manifest.terminal_id),
        &marker,
    )
    .unwrap();
    (directory, host, manifest, store, transaction, marker)
}
