use super::*;

#[tokio::test]
async fn recovery_requires_settled_processes_and_matching_exit_epoch() {
    let home = tempdir().unwrap();
    let first = Host::open(HostConfig::new(home.path())).unwrap();
    let mut manifest = fixture_manifest(home.path());
    manifest.archived = true;
    let id = manifest.terminal_id;
    seed_recovery_manifest(&first, &manifest).await;
    let root = stage(&first, id);
    let live = host_runtime_record(&first, id, home.path().join("worker.sock"));
    persist_archive_runtime(&first, &live);
    first
        .write_namespace_json(
            &first.paths.exit_record(id),
            &archive_exit_for(live.runtime_epoch),
        )
        .unwrap();
    drop(first);

    let host = Host::open(HostConfig::new(home.path())).unwrap();
    assert_eq!(host.retry_archived_attachment_cleanup().await, 1);
    assert!(root.is_dir());
    let replacement = dead_archive_record(&host, id, home.path().join("replacement.sock"));
    persist_archive_runtime(&host, &replacement);
    assert_eq!(host.retry_archived_attachment_cleanup().await, 1);
    assert!(root.is_dir());
    host.write_namespace_json(
        &host.paths.exit_record(id),
        &archive_exit_for(replacement.runtime_epoch),
    )
    .unwrap();
    assert_eq!(host.retry_archived_attachment_cleanup().await, 0);
    assert!(!root.exists());
}

#[tokio::test]
async fn retry_rechecks_durable_archive_and_original_directory() {
    let home = tempdir().unwrap();
    let first = Host::open(HostConfig::new(home.path())).unwrap();
    let mut archived = fixture_manifest(home.path());
    archived.archived = true;
    seed_recovery_manifest(&first, &archived).await;
    let root = stage(&first, archived.terminal_id);
    let mut replaced = fixture_manifest(home.path());
    replaced.archived = true;
    seed_recovery_manifest(&first, &replaced).await;
    stage(&first, replaced.terminal_id);
    let mut removed = fixture_manifest(home.path());
    removed.archived = true;
    seed_recovery_manifest(&first, &removed).await;
    stage(&first, removed.terminal_id);
    drop(first);

    let host = Host::open(HostConfig::new(home.path())).unwrap();
    archived.archived = false;
    host.write_namespace_json(&host.paths.manifest(archived.terminal_id), &archived)
        .unwrap();
    let id = replaced.terminal_id;
    let removed_path = home.path().join("removed-session");
    fs::rename(host.paths.session_dir(removed.terminal_id), &removed_path).unwrap();
    let moved = home.path().join("old-session");
    fs::rename(host.paths.session_dir(id), &moved).unwrap();
    host.paths.ensure_session(id).unwrap();
    host.write_namespace_json(&host.paths.manifest(id), &replaced)
        .unwrap();
    let replacement = stage(&host, id);
    assert_eq!(host.retry_archived_attachment_cleanup().await, 0);
    assert!(root.is_dir());
    assert!(replacement.is_dir());
    assert!(directory(&moved).is_dir());
    assert!(directory(&removed_path).is_dir());
    assert!(lock_recover(&host.pending_attachment_cleanup).is_empty());
}

#[tokio::test]
async fn retry_preserves_a_replacement_attachment_root_inside_the_same_archived_session() {
    let home = tempdir().unwrap();
    let first = Host::open(HostConfig::new(home.path())).unwrap();
    let mut manifest = fixture_manifest(home.path());
    manifest.archived = true;
    seed_recovery_manifest(&first, &manifest).await;
    let root = stage(&first, manifest.terminal_id);
    drop(first);
    let host = Host::open(HostConfig::new(home.path())).unwrap();
    let original = home.path().join("original-uploads");
    fs::rename(&root, &original).unwrap();
    host.namespace.ensure_directory(&root).unwrap();
    let replacement = root.join("replacement");
    write_private_bytes(&replacement, HISTORY_BYTES);
    assert_eq!(host.retry_archived_attachment_cleanup().await, 0);
    assert_eq!(fs::read(replacement).unwrap(), HISTORY_BYTES);
    assert!(original.is_dir());
}
