use crate::worker::tests::*;
use std::os::unix::fs::PermissionsExt;
const MALFORMED_SHUTDOWN_MANIFEST: &[u8] = b"{";
const EXIT_TIMESTAMP_MUTATION_MS: i64 = 1;

#[tokio::test]
async fn committed_projection_drain_defers_revoked_manifest_to_runtime_settlement() {
    let fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&fixture);
    fixture.state.commit_response_shutdown();

    codex_hook::supervise_closed_projection_for_test(Arc::clone(&fixture.state))
        .await
        .unwrap();
    settle_worker_runtime_with_projection(
        &fixture.state,
        JoinSet::<()>::new(),
        Ok(()),
        AgentProjectionShutdown::ManifestRevoked(removed),
    )
    .await
    .unwrap();

    assert_projection_active(&fixture);
    assert!(
        !fixture
            .state
            .paths
            .manifest(fixture.terminal_id())
            .try_exists()
            .unwrap()
    );
}

#[tokio::test]
async fn durable_manifest_revocation_makes_shutdown_projection_stale() {
    let fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&fixture);

    let changed = codex_hook::invalidate_projection_for_shutdown(
        &fixture.state,
        AgentProjectionShutdown::ManifestRevoked(removed),
    )
    .await
    .unwrap();

    assert!(!changed);
    assert_projection_active(&fixture);
    assert!(
        fixture
            .state
            .namespace
            .file_is_missing(&fixture.state.paths.manifest(fixture.terminal_id()))
            .unwrap()
    );
}
#[tokio::test]
async fn concurrent_manifest_removal_cannot_weaken_ordinary_shutdown() {
    let fixture = active_projection_fixture();
    let gate = fixture.state.agent_generation_gate.lock().await;
    let state = Arc::clone(&fixture.state);
    let shutdown = tokio::spawn(async move {
        codex_hook::invalidate_projection_for_shutdown(&state, AgentProjectionShutdown::Invalidate)
            .await
    });
    tokio::task::yield_now().await;
    assert!(!shutdown.is_finished());
    std::fs::remove_file(fixture.state.paths.manifest(fixture.terminal_id())).unwrap();
    drop(gate);

    assert!(shutdown.await.unwrap().is_err());
    assert_projection_active(&fixture);
}
#[tokio::test]
async fn revoked_shutdown_rejects_runtime_replacement() {
    let fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&fixture);
    let current = lock_recover(&fixture.state.record).clone();
    atomic_write_json(
        &fixture.state.paths.runtime_record(fixture.terminal_id()),
        &successor_runtime(&current),
    )
    .unwrap();

    assert_revoked_shutdown_rejected(&fixture, removed).await;
}
#[tokio::test]
async fn revoked_shutdown_rejects_malformed_or_wrong_session_manifests() {
    let fixture = active_projection_fixture();
    let (removed, mut manifest) = prepare_revoked_projection(&fixture);
    let path = fixture.state.paths.manifest(fixture.terminal_id());
    std::fs::write(&path, MALFORMED_SHUTDOWN_MANIFEST).unwrap();
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    assert_revoked_shutdown_rejected(&fixture, removed).await;

    std::fs::remove_file(&path).unwrap();
    let removed = RemovedTerminalManifest { path: path.clone() };
    manifest.terminal_id = Uuid::new_v4();
    atomic_write_json(&path, &manifest).unwrap();
    assert_revoked_shutdown_rejected(&fixture, removed).await;
}

#[tokio::test]
async fn revoked_shutdown_rejects_manifest_io_and_token_identity_faults() {
    let fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&fixture);
    let path = fixture.state.paths.manifest(fixture.terminal_id());
    std::fs::create_dir(&path).unwrap();
    assert_revoked_shutdown_rejected(&fixture, removed).await;

    std::fs::remove_dir(&path).unwrap();
    let wrong = RemovedTerminalManifest {
        path: fixture.state.paths.manifest(Uuid::new_v4()),
    };
    assert_revoked_shutdown_rejected(&fixture, wrong).await;
}

#[tokio::test]
async fn revoked_shutdown_requires_exact_durable_lost_exit() {
    let missing = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&missing);
    std::fs::remove_file(missing.state.paths.exit_record(missing.terminal_id())).unwrap();
    assert_revoked_shutdown_rejected(&missing, removed).await;

    let mismatched = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&mismatched);
    let mut durable: ExitRecord =
        read_json(&mismatched.state.paths.exit_record(mismatched.terminal_id())).unwrap();
    durable.observed_at_ms += EXIT_TIMESTAMP_MUTATION_MS;
    atomic_write_json(
        &mismatched.state.paths.exit_record(mismatched.terminal_id()),
        &durable,
    )
    .unwrap();
    assert_revoked_shutdown_rejected(&mismatched, removed).await;

    for mutate in [
        |exit: &mut ExitRecord| exit.reason = ExitReason::Natural,
        |exit: &mut ExitRecord| exit.runtime_epoch = Uuid::new_v4(),
    ] {
        let fixture = active_projection_fixture();
        let (removed, _) = prepare_revoked_projection(&fixture);
        let mut exit = lock_recover(&fixture.state.exit).clone().unwrap();
        mutate(&mut exit);
        fixture
            .state
            .persist_json(
                &fixture.state.paths.exit_record(fixture.terminal_id()),
                &exit,
            )
            .unwrap();
        *lock_recover(&fixture.state.exit) = Some(exit);
        assert_revoked_shutdown_rejected(&fixture, removed).await;
    }
}

pub(super) fn prepare_revoked_projection(
    fixture: &Fixture,
) -> (RemovedTerminalManifest, crate::model::TerminalManifest) {
    let path = fixture.state.paths.manifest(fixture.terminal_id());
    let manifest = read_json(&path).unwrap();
    let exit = ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: None,
        signal: None,
        reason: ExitReason::Lost,
        observed_at_ms: now_ms(),
        last_output_sequence: lock_recover(&fixture.state.journal).last_sequence(),
    };
    fixture
        .state
        .persist_json(
            &fixture.state.paths.exit_record(fixture.terminal_id()),
            &exit,
        )
        .unwrap();
    *lock_recover(&fixture.state.exit) = Some(exit);
    std::fs::remove_file(&path).unwrap();
    (RemovedTerminalManifest { path }, manifest)
}

pub(super) async fn assert_revoked_shutdown_rejected(
    fixture: &Fixture,
    removed: RemovedTerminalManifest,
) {
    assert!(
        codex_hook::invalidate_projection_for_shutdown(
            &fixture.state,
            AgentProjectionShutdown::ManifestRevoked(removed),
        )
        .await
        .is_err()
    );
    assert_projection_active(fixture);
}
