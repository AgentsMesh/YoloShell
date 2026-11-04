use crate::host::tests::*;
use super::*;

const LIFECYCLE_SELECTION_SOCKET_EXTENSION: &str = "sock";
const LIFECYCLE_SELECTION_TIMESTAMP_INCREMENT: i64 = 1;

fn lifecycle_selection(
    terminal_id: Uuid,
    updated_at_ms: Option<i64>,
    status: Option<TerminalRuntimeStatus>,
) -> TerminalLifecycleSelection {
    TerminalLifecycleSelection {
        terminal_id,
        expected_updated_at_ms: updated_at_ms,
        expected_status: status,
        expected_manifest_sha256: None,
        expected_runtime_epoch: None,
        require_runtime_absent: false,
    }
}

#[tokio::test]
async fn convergence_product_selection_checks_absence_time_and_status() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let missing = lifecycle_selection(Uuid::new_v4(), None, None);
    assert_eq!(
        host.validate_product_selection(&missing)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::NOT_FOUND
    );

    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    host.validate_product_selection(&lifecycle_selection(manifest.terminal_id, None, None))
        .await
        .unwrap();
    let stale = lifecycle_selection(
        manifest.terminal_id,
        Some(manifest.updated_at_ms + LIFECYCLE_SELECTION_TIMESTAMP_INCREMENT),
        None,
    );
    assert_eq!(
        host.validate_product_selection(&stale)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );

    let mut archived = fixture_manifest(directory.path());
    archived.archived = true;
    seed_recovery_manifest(&host, &archived).await;
    let wrong = lifecycle_selection(
        archived.terminal_id,
        Some(archived.updated_at_ms),
        Some(TerminalRuntimeStatus::Running),
    );
    assert!(host.validate_product_selection(&wrong).await.is_err());

    let unreachable = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &unreachable).await;
    let selected = lifecycle_selection(
        unreachable.terminal_id,
        None,
        Some(TerminalRuntimeStatus::Unreachable),
    );
    host.validate_product_selection(&selected).await.unwrap();
}

#[tokio::test]
async fn convergence_product_selection_rechecks_catalog_after_runtime_status() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let socket = directory
        .path()
        .join(manifest.terminal_id.to_string())
        .with_extension(LIFECYCLE_SELECTION_SOCKET_EXTENSION);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, manifest.terminal_id, socket);
    let status = status_view(&record, Some(Vec::new()));
    persist_host_session(&host, &manifest, &record);
    let catalog = Arc::clone(&host.catalog);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        let mut changed = lock_recover(&catalog)
            .get(record.terminal_id)
            .unwrap()
            .unwrap();
        changed.updated_at_ms += LIFECYCLE_SELECTION_TIMESTAMP_INCREMENT;
        lock_recover(&catalog).upsert(&changed).unwrap();
        let response = ResponseEnvelope::success(&request, &status).unwrap();
        stream.respond(&response).await.unwrap();
    });
    let selected = lifecycle_selection(
        manifest.terminal_id,
        Some(manifest.updated_at_ms),
        Some(TerminalRuntimeStatus::Running),
    );

    assert_eq!(
        host.validate_product_selection(&selected)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
    server.await.unwrap();
}

#[tokio::test]
async fn epoch_fenced_discard_rejects_a_rearmed_runtime_without_touching_it() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    let missing =
        TerminalLifecycleSelection::at_runtime_epoch(manifest.terminal_id, Uuid::new_v4());
    assert_eq!(
        host.validate_runtime_epoch_fence(&missing)
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
    let original = host_runtime_record(
        &host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    persist_host_session(&host, &manifest, &original);
    let captured_epoch = original.runtime_epoch;
    let mut rearmed = original;
    rearmed.runtime_epoch = Uuid::new_v4();
    rearmed.worker_id = Uuid::new_v4();
    atomic_write_json(&runtime_path, &rearmed).unwrap();
    let stale = TerminalLifecycleSelection::at_runtime_epoch(manifest.terminal_id, captured_epoch);

    let terminate_error = host
        .terminate_product_request(TerminalTerminateRequest {
            selection: stale.clone(),
            force: true,
        })
        .await
        .unwrap_err();
    let archive_error = host
        .archive_product_session(stale.clone())
        .await
        .unwrap_err();
    for error in [terminate_error, archive_error] {
        assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    }

    let error = host
        .discard_product_session(TerminalDiscardRequest {
            selection: stale,
            force: true,
        })
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(read_json::<RuntimeRecord>(&runtime_path).unwrap(), rearmed);
    assert!(
        !host
            .catalog_get(manifest.terminal_id)
            .await
            .unwrap()
            .unwrap()
            .archived
    );
    assert!(
        !host
            .paths
            .terminate_tombstone(manifest.terminal_id)
            .exists()
    );
}
