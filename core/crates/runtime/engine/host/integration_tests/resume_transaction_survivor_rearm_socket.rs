use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn dead_survivor_rearm_preserves_t1_while_session_socket_is_serving() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (manifest, transaction, runtime) = dead_survivor_rearm_fixture(&host, directory.path());
    let tombstone = read_resume_tombstone(&host, manifest.terminal_id);
    let (_listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;

    let error = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    drop(publication);

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(manifest.terminal_id)).unwrap(),
        transaction.launch
    );
    assert_eq!(read_resume_tombstone(&host, manifest.terminal_id), tombstone);
    assert_eq!(
        host.load_durable_manifest(manifest.terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
}
