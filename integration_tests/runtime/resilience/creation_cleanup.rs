use super::*;

fn assert_cleanup_error(error: ClientError, expected: &str) {
    assert!(matches!(
        error,
        ClientError::Remote { code, .. } if code == expected
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cleanup_rejects_wrong_capabilities_and_replays_phases_after_restart() {
    let mut harness = Harness::start().await;
    let view = harness.create_unfinalized_cat("cleanup-phases").await;
    let terminal_id = view.manifest.terminal_id;
    let (token, capability) = harness.creation_credentials(terminal_id);
    let wrong = Uuid::new_v4();
    assert_cleanup_error(
        harness
            .client()
            .cleanup_terminate(terminal_id, wrong, true)
            .await
            .unwrap_err(),
        error_code::UNAUTHORIZED,
    );
    assert_cleanup_error(
        harness
            .client()
            .cleanup_archive(terminal_id, wrong)
            .await
            .unwrap_err(),
        error_code::UNAUTHORIZED,
    );
    assert_cleanup_error(
        harness
            .client()
            .cleanup_discard(terminal_id, wrong, true)
            .await
            .unwrap_err(),
        error_code::UNAUTHORIZED,
    );
    assert_eq!(
        harness
            .client()
            .get_session(terminal_id)
            .await
            .unwrap()
            .runtime
            .unwrap()
            .status,
        TerminalRuntimeStatus::Running
    );
    assert_cleanup_error(
        harness
            .client()
            .cleanup_archive(terminal_id, capability)
            .await
            .unwrap_err(),
        error_code::CONFLICT,
    );

    harness
        .client()
        .cleanup_terminate(terminal_id, capability, true)
        .await
        .unwrap();
    harness
        .client()
        .cleanup_terminate(terminal_id, capability, true)
        .await
        .unwrap();
    harness.restart_host_hard().await;
    let archived = harness
        .client()
        .cleanup_archive(terminal_id, capability)
        .await
        .unwrap();
    assert!(archived.manifest.archived);
    restart_after_cleanup_phase(&mut harness).await;
    assert!(
        harness
            .client()
            .cleanup_archive(terminal_id, capability)
            .await
            .unwrap()
            .manifest
            .archived
    );
    restart_after_cleanup_phase(&mut harness).await;
    assert!(
        harness
            .client()
            .cleanup_discard(terminal_id, capability, true)
            .await
            .unwrap()
            .manifest
            .archived
    );
    assert!(
        harness
            .client()
            .finalize_creation(token, terminal_id, capability)
            .await
            .is_err()
    );
}

async fn restart_after_cleanup_phase(harness: &mut Harness) {
    harness.stop_host_with_sigterm();
    harness.replace_host(spawn_host(harness.home.path())).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ordinary_termination_prevents_transaction_finalize() {
    let harness = Harness::start().await;
    let view = harness.create_unfinalized_cat("ordinary-termination").await;
    let terminal_id = view.manifest.terminal_id;
    let (token, capability) = harness.creation_credentials(terminal_id);
    harness.client().terminate(terminal_id, true).await.unwrap();
    assert_cleanup_error(
        harness
            .client()
            .finalize_creation(token, terminal_id, capability)
            .await
            .unwrap_err(),
        error_code::CONFLICT,
    );
    assert!(
        harness
            .client()
            .discard(terminal_id, true)
            .await
            .unwrap()
            .manifest
            .archived
    );
}
