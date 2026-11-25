use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn archived_sessions_cannot_be_resumed() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.archived = true;
    seed_recovery_manifest(&host, &manifest).await;
    let error = host
        .resume_session_locked(manifest.terminal_id, ResumeSelection::explicit(false))
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
}

#[tokio::test]
async fn durable_only_archive_is_not_reported_as_missing() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.archived = true;
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &manifest)
        .unwrap();

    let error = host
        .resume_session_locked(manifest.terminal_id, ResumeSelection::explicit(false))
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
}

#[tokio::test]
async fn durable_archive_wins_over_a_stale_catalog_row() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let terminal_id = manifest.terminal_id;
    seed_recovery_manifest(&host, &manifest).await;
    let mut archived = manifest.clone();
    archived.archived = true;
    archived.updated_at_ms = archived.updated_at_ms.saturating_add(1);
    host.write_namespace_json(&host.paths.manifest(terminal_id), &archived)
        .unwrap();
    assert!(
        !host
            .catalog_get(terminal_id)
            .await
            .unwrap()
            .unwrap()
            .archived
    );

    let error = host
        .resume_session_locked(terminal_id, ResumeSelection::explicit(false))
        .await
        .unwrap_err()
        .into_explicit_error();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert!(durable.archived);
}

#[tokio::test]
async fn stale_catalog_row_cannot_overwrite_the_durable_manifest_on_resume() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let terminal_id = manifest.terminal_id;
    seed_recovery_manifest(&host, &manifest).await;
    let mut durable = manifest.clone();
    durable.name = format!("{}-durable", durable.name);
    durable.updated_at_ms = durable.updated_at_ms.saturating_add(1);
    host.write_namespace_json(&host.paths.manifest(terminal_id), &durable)
        .unwrap();

    // The fixture cannot spawn a Worker; the point is that the stale catalog
    // name never replaces the durable name.
    let _error = host
        .resume_session_locked(terminal_id, ResumeSelection::explicit(false))
        .await
        .unwrap_err()
        .into_explicit_error();
    let current: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert_eq!(current.name, durable.name);
}
